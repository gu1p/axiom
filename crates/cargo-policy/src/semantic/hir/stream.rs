use std::io::{BufRead as _, BufReader};
use std::process::{Child, ChildStdout, Command, Stdio};

use policy_core::{AnalysisError, AnalysisInput, CodebaseFacts};
use serde::Deserialize;

use super::append_diagnostic;
use crate::process::{configure_group, terminate_group};
use crate::semantic::types::{RawSemanticDiagnostic, SemanticReport};

const STREAM_SCHEMA: u32 = 1;

pub(super) enum Outcome {
    Complete(SemanticReport),
    Stopped,
}

#[derive(Deserialize)]
struct Event {
    stream_schema_version: u32,
    #[serde(rename = "event")]
    kind: String,
    diagnostic: Option<RawSemanticDiagnostic>,
    report: Option<SemanticReport>,
}

enum ReadOutcome {
    Complete(Option<SemanticReport>),
    Stopped,
}

pub(super) fn run(
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
    command: &mut Command,
    stop_on_private: &mut dyn FnMut(&CodebaseFacts) -> bool,
) -> Result<Outcome, AnalysisError> {
    let (mut child, stderr, stdout) = start(command)?;
    let read = match read_events(input, facts, stdout, stop_on_private) {
        Ok(read) => read,
        Err(error) => {
            stop_process(&mut child)?;
            return Err(error);
        }
    };
    match read {
        ReadOutcome::Stopped => {
            stop_process(&mut child)?;
            Ok(Outcome::Stopped)
        }
        ReadOutcome::Complete(report) => finish(child, &stderr, report),
    }
}

fn start(
    command: &mut Command,
) -> Result<(Child, tempfile::NamedTempFile, ChildStdout), AnalysisError> {
    configure_group(command);
    let stderr = tempfile::NamedTempFile::new().map_err(|error| {
        AnalysisError::new(format!("could not capture semantic analysis: {error}"))
    })?;
    let stderr_writer = stderr.reopen().map_err(|error| {
        AnalysisError::new(format!("could not capture semantic analysis: {error}"))
    })?;
    command.stdout(Stdio::piped()).stderr(stderr_writer);
    let mut child = command
        .spawn()
        .map_err(|error| AnalysisError::new(format!("could not run semantic analysis: {error}")))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        AnalysisError::new("could not capture streaming semantic analysis output")
    })?;
    Ok((child, stderr, stdout))
}

fn read_events(
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
    stdout: ChildStdout,
    stop_on_private: &mut dyn FnMut(&CodebaseFacts) -> bool,
) -> Result<ReadOutcome, AnalysisError> {
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| {
            AnalysisError::new(format!("could not read semantic analysis output: {error}"))
        })?;
        let event = parse_event(&line)?;
        if let Some(outcome) = handle_event(event, input, facts, stop_on_private)? {
            return Ok(outcome);
        }
    }
    Ok(ReadOutcome::Complete(None))
}

fn parse_event(line: &str) -> Result<Event, AnalysisError> {
    let event: Event = serde_json::from_str(line).map_err(|error| {
        AnalysisError::new(format!(
            "semantic analyzer returned invalid stream JSON: {error}"
        ))
    })?;
    if event.stream_schema_version != STREAM_SCHEMA {
        return Err(AnalysisError::new(format!(
            "semantic stream schema {} is incompatible with expected schema {STREAM_SCHEMA}",
            event.stream_schema_version
        )));
    }
    Ok(event)
}

fn handle_event(
    event: Event,
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
    stop_on_private: &mut dyn FnMut(&CodebaseFacts) -> bool,
) -> Result<Option<ReadOutcome>, AnalysisError> {
    match event.kind.as_str() {
        "private_dead_code" => {
            let diagnostic = event.diagnostic.ok_or_else(|| {
                AnalysisError::new("semantic private-dead-code event has no diagnostic")
            })?;
            let before = facts.semantic_findings.len();
            append_diagnostic(input, facts, diagnostic)?;
            let stopped = facts.semantic_findings.len() != before && stop_on_private(facts);
            Ok(stopped.then_some(ReadOutcome::Stopped))
        }
        "complete" => Ok(Some(ReadOutcome::Complete(event.report))),
        other => Err(AnalysisError::new(format!(
            "semantic analyzer returned unknown stream event `{other}`"
        ))),
    }
}

fn finish(
    mut child: Child,
    stderr: &tempfile::NamedTempFile,
    report: Option<SemanticReport>,
) -> Result<Outcome, AnalysisError> {
    let status = child.wait().map_err(|error| {
        AnalysisError::new(format!("could not wait for semantic analysis: {error}"))
    })?;
    if !status.success() {
        let detail = std::fs::read_to_string(stderr.path()).unwrap_or_default();
        return Err(AnalysisError::new(if detail.trim().is_empty() {
            format!("semantic analysis failed with {status}")
        } else {
            format!("semantic analysis failed: {}", detail.trim())
        }));
    }
    if let Some(report) = report {
        Ok(Outcome::Complete(report))
    } else {
        Err(AnalysisError::new(
            "semantic analysis stream ended before completion",
        ))
    }
}

fn stop_process(child: &mut std::process::Child) -> Result<(), AnalysisError> {
    terminate_group(child)
}
