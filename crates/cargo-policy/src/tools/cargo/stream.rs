use std::io::BufReader;
use std::process::{Child, ChildStdout, Command, Stdio};

use cargo_metadata::Message;
use policy_core::{AnalysisError, AnalysisInput, Level};

use super::{command_error, level, tool_diagnostic};
use crate::process::{configure_group, terminate_group};
use crate::tools::{ToolDiagnostic, ToolReport};

pub(super) fn execute(
    tool: &'static str,
    input: &AnalysisInput,
    command: &mut Command,
    ignore_warnings: bool,
) -> Result<ToolReport, AnalysisError> {
    let (mut child, stderr, stdout) = start(tool, command)?;
    let diagnostic = match first_diagnostic(tool, input, stdout, ignore_warnings) {
        Ok(diagnostic) => diagnostic,
        Err(error) => {
            terminate_group(&mut child)?;
            return Err(error);
        }
    };
    if let Some(diagnostic) = diagnostic {
        terminate_group(&mut child)?;
        return Ok(ToolReport {
            name: tool,
            diagnostics: vec![diagnostic],
        });
    }
    finish(tool, child, &stderr)
}

fn start(
    tool: &str,
    command: &mut Command,
) -> Result<(Child, tempfile::NamedTempFile, ChildStdout), AnalysisError> {
    configure_group(command);
    let stderr = tempfile::NamedTempFile::new()
        .map_err(|error| AnalysisError::new(format!("could not capture {tool}: {error}")))?;
    let stderr_writer = stderr
        .reopen()
        .map_err(|error| AnalysisError::new(format!("could not capture {tool}: {error}")))?;
    command.stdout(Stdio::piped()).stderr(stderr_writer);
    let mut child = command.spawn().map_err(|error| {
        AnalysisError::new(format!("could not run {tool} through Cargo: {error}"))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AnalysisError::new(format!("could not capture Cargo {tool} output")))?;
    Ok((child, stderr, stdout))
}

fn first_diagnostic(
    tool: &'static str,
    input: &AnalysisInput,
    stdout: ChildStdout,
    ignore_warnings: bool,
) -> Result<Option<ToolDiagnostic>, AnalysisError> {
    for message in Message::parse_stream(BufReader::new(stdout)) {
        let message = message.map_err(|error| {
            AnalysisError::new(format!("could not parse Cargo's {tool} output: {error}"))
        })?;
        let Message::CompilerMessage(message) = message else {
            continue;
        };
        let Some(diagnostic_level) = level(message.message.level) else {
            continue;
        };
        if ignore_warnings && diagnostic_level == Level::Warn {
            continue;
        }
        let diagnostic = tool_diagnostic(tool, input, message.message, diagnostic_level);
        return Ok(Some(diagnostic));
    }
    Ok(None)
}

fn finish(
    tool: &'static str,
    mut child: Child,
    stderr: &tempfile::NamedTempFile,
) -> Result<ToolReport, AnalysisError> {
    let status = child
        .wait()
        .map_err(|error| AnalysisError::new(format!("could not wait for {tool}: {error}")))?;
    if !status.success() {
        let detail = std::fs::read_to_string(stderr.path()).unwrap_or_default();
        return Err(command_error(tool, status, detail.trim()));
    }
    Ok(ToolReport {
        name: tool,
        diagnostics: Vec::new(),
    })
}
