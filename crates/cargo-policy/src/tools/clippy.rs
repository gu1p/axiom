use std::collections::BTreeSet;
use std::env;
use std::io::BufReader;
use std::process::Command;

use camino::Utf8PathBuf;
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel, DiagnosticSpan};
use cargo_metadata::{Message, diagnostic};
use policy_core::{AnalysisError, AnalysisInput, ClippyConfig, Level, Position, SourceSpan};

use super::{ToolDiagnostic, ToolReport};

const TOOL_NAME: &str = "clippy";

pub fn run(input: &AnalysisInput, config: &ClippyConfig) -> Result<ToolReport, AnalysisError> {
    let output = command(input, config).output().map_err(|error| {
        AnalysisError::new(format!(
            "could not run Clippy through Cargo: {error}; install it with `rustup component add clippy`"
        ))
    })?;
    let diagnostics = parse_diagnostics(input, &output.stdout)?;
    if !output.status.success()
        && !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == Level::Deny)
    {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        return Err(AnalysisError::new(if detail.is_empty() {
            format!("Clippy could not complete: {}", output.status)
        } else {
            format!("Clippy could not complete: {detail}")
        }));
    }
    Ok(ToolReport {
        name: TOOL_NAME,
        diagnostics,
    })
}

fn command(input: &AnalysisInput, config: &ClippyConfig) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(&input.workspace_root)
        .arg("clippy")
        .arg("--manifest-path")
        .arg(input.workspace_root.join("Cargo.toml"))
        .args([
            "--workspace",
            "--locked",
            "--no-deps",
            "--keep-going",
            "--quiet",
            "--message-format=json",
            "--color=never",
        ]);
    if config.checks_all_targets() {
        command.arg("--all-targets");
    }
    if config.checks_all_features() {
        command.arg("--all-features");
    } else if let Some(features) = config.selected_features() {
        command.arg("--features").arg(features.join(","));
    }
    if config.no_default_features {
        command.arg("--no-default-features");
    }
    if config.denies_warnings() {
        command.args(["--", "-D", "warnings"]);
    }
    if offline() {
        command.env("CARGO_NET_OFFLINE", "true");
    }
    command
}

fn parse_diagnostics(
    input: &AnalysisInput,
    output: &[u8],
) -> Result<Vec<ToolDiagnostic>, AnalysisError> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    for message in Message::parse_stream(BufReader::new(output)) {
        let message = message.map_err(|error| {
            AnalysisError::new(format!("could not parse Cargo's Clippy output: {error}"))
        })?;
        let Message::CompilerMessage(message) = message else {
            continue;
        };
        let Some(level) = level(message.message.level) else {
            continue;
        };
        let diagnostic = tool_diagnostic(input, message.message, level);
        let identity = (
            diagnostic.rule_id.clone(),
            diagnostic.message.clone(),
            diagnostic.path.clone(),
            diagnostic.span.map(|span| span.byte_start),
            diagnostic.level == Level::Deny,
        );
        if seen.insert(identity) {
            diagnostics.push(diagnostic);
        }
    }
    Ok(diagnostics)
}

fn level(level: DiagnosticLevel) -> Option<Level> {
    match level {
        DiagnosticLevel::Warning => Some(Level::Warn),
        DiagnosticLevel::Ice | DiagnosticLevel::Error | DiagnosticLevel::FailureNote => {
            Some(Level::Deny)
        }
        _ => None,
    }
}

fn tool_diagnostic(input: &AnalysisInput, diagnostic: Diagnostic, level: Level) -> ToolDiagnostic {
    let location = diagnostic
        .spans
        .iter()
        .find(|span| span.is_primary)
        .map(|span| source_location(input, span));
    ToolDiagnostic {
        tool: TOOL_NAME,
        rule_id: diagnostic
            .code
            .as_ref()
            .map_or_else(|| TOOL_NAME.to_owned(), |code| code.code.clone()),
        level,
        message: diagnostic.message.clone(),
        help: diagnostic
            .children
            .iter()
            .find(|child| child.level == diagnostic::DiagnosticLevel::Help)
            .map(|child| child.message.clone()),
        path: location.as_ref().map(|(path, _)| path.clone()),
        span: location.map(|(_, span)| span),
        rendered: diagnostic.rendered,
    }
}

fn source_location(input: &AnalysisInput, span: &DiagnosticSpan) -> (Utf8PathBuf, SourceSpan) {
    let mut path = Utf8PathBuf::from(span.file_name.replace('\\', "/"));
    if path.is_absolute()
        && let Ok(relative) = path.strip_prefix(&input.workspace_root)
    {
        path = relative.to_owned();
    }
    (
        path,
        SourceSpan {
            byte_start: span.byte_start,
            byte_end: span.byte_end,
            start: Position {
                line: to_u32(span.line_start),
                column: to_u32(span.column_start),
            },
            end: Position {
                line: to_u32(span.line_end),
                column: to_u32(span.column_end),
            },
        },
    )
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn offline() -> bool {
    env::var("AXIOM_OFFLINE")
        .is_ok_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

#[cfg(test)]
#[path = "tests/clippy.rs"]
mod tests;
