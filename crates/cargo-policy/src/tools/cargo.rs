use std::collections::BTreeSet;
use std::io::BufReader;
use std::process::Command;

use camino::Utf8PathBuf;
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel, DiagnosticSpan};
use cargo_metadata::{Message, diagnostic};
use policy_core::{AnalysisError, AnalysisInput, Level, Position, SourceSpan};

use super::{ToolDiagnostic, ToolReport};

pub fn execute(
    tool: &'static str,
    input: &AnalysisInput,
    command: &mut Command,
) -> Result<ToolReport, AnalysisError> {
    let output = command.output().map_err(|error| {
        AnalysisError::new(format!("could not run {tool} through Cargo: {error}"))
    })?;
    let diagnostics = parse_diagnostics(tool, input, &output.stdout)?;
    if !output.status.success()
        && !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == Level::Deny)
    {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(command_error(tool, output.status, detail.trim()));
    }
    Ok(ToolReport {
        name: tool,
        diagnostics,
    })
}

fn parse_diagnostics(
    tool: &'static str,
    input: &AnalysisInput,
    output: &[u8],
) -> Result<Vec<ToolDiagnostic>, AnalysisError> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    for message in Message::parse_stream(BufReader::new(output)) {
        let message = message.map_err(|error| {
            AnalysisError::new(format!("could not parse Cargo's {tool} output: {error}"))
        })?;
        let Message::CompilerMessage(message) = message else {
            continue;
        };
        let Some(level) = level(message.message.level) else {
            continue;
        };
        let diagnostic = tool_diagnostic(tool, input, message.message, level);
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

fn tool_diagnostic(
    tool: &'static str,
    input: &AnalysisInput,
    diagnostic: Diagnostic,
    level: Level,
) -> ToolDiagnostic {
    let location = diagnostic
        .spans
        .iter()
        .find(|span| span.is_primary)
        .map(|span| source_location(input, span));
    ToolDiagnostic {
        tool,
        rule_id: diagnostic
            .code
            .as_ref()
            .map_or_else(|| tool.to_owned(), |code| code.code.clone()),
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

fn command_error(tool: &str, status: std::process::ExitStatus, detail: &str) -> AnalysisError {
    if detail.is_empty() {
        AnalysisError::new(format!("{tool} could not complete: {status}"))
    } else {
        AnalysisError::new(format!("{tool} could not complete: {detail}"))
    }
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
