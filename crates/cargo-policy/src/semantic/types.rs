use std::process::Output;

use camino::Utf8PathBuf;
use policy_core::{
    AnalysisError, AnalysisInput, CodebaseFacts, Position, SemanticFindingFact, SourceSpan,
};
use serde::Deserialize;

pub(super) fn source_location(
    input: &AnalysisInput,
    location: &RawLocation,
) -> Option<(Utf8PathBuf, SourceSpan)> {
    let mut path = Utf8PathBuf::from(location.file.replace('\\', "/"));
    if path.is_absolute() {
        path = path.strip_prefix(&input.workspace_root).ok()?.to_owned();
    }
    let source = input
        .sources
        .iter()
        .find(|source| source.relative_path == path)?;
    let start = location.byte_start.unwrap_or(0);
    let end = location.byte_end.unwrap_or(start.saturating_add(1));
    let span = location.byte_start.map_or_else(
        || {
            source
                .lines
                .span(&source.text, 0, usize::from(!source.text.is_empty()))
        },
        |_| location.span(start, end),
    );
    Some((path, span))
}

pub(super) fn push_unique(facts: &mut CodebaseFacts, finding: SemanticFindingFact) {
    if facts.semantic_findings.iter().any(|existing| {
        existing.kind == finding.kind
            && existing.path == finding.path
            && existing.span.byte_start == finding.span.byte_start
    }) {
        return;
    }
    facts.semantic_findings.push(finding);
}

pub(super) fn command_failure(operation: &str, output: &Output) -> AnalysisError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr.trim();
    AnalysisError::new(if detail.is_empty() {
        format!("{operation} failed with {}", output.status)
    } else {
        format!("{operation} failed: {detail}")
    })
}

#[derive(Deserialize)]
pub(super) struct SemanticReport {
    pub schema_version: u32,
    pub diagnostics: Vec<RawSemanticDiagnostic>,
}

#[derive(Deserialize)]
pub(super) struct RawSemanticDiagnostic {
    pub category: String,
    pub code: String,
    pub identity: Option<RawIdentity>,
    pub location: Option<RawLocation>,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct RawIdentity {
    pub item: String,
    pub kind: String,
}

#[derive(Deserialize)]
pub(super) struct RawLocation {
    pub file: String,
    pub byte_start: Option<u32>,
    pub byte_end: Option<u32>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
}

impl RawLocation {
    fn span(&self, start: u32, end: u32) -> SourceSpan {
        SourceSpan {
            byte_start: start,
            byte_end: end,
            start: Position {
                line: self.line.unwrap_or(1),
                column: self.column.unwrap_or(1),
            },
            end: Position {
                line: self.end_line.unwrap_or_else(|| self.line.unwrap_or(1)),
                column: self
                    .end_column
                    .unwrap_or_else(|| self.column.unwrap_or(1).saturating_add(1)),
            },
        }
    }
}

impl From<RustcSpan> for RawLocation {
    fn from(span: RustcSpan) -> Self {
        Self {
            file: span.file_name,
            byte_start: Some(span.byte_start),
            byte_end: Some(span.byte_end),
            line: Some(span.line_start),
            column: Some(span.column_start),
            end_line: Some(span.line_end),
            end_column: Some(span.column_end),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct CargoMessage {
    #[serde(default)]
    pub message: Option<RustcDiagnostic>,
}

#[derive(Deserialize)]
pub(super) struct RustcDiagnostic {
    pub message: String,
    pub level: String,
    pub code: Option<RustcCode>,
    #[serde(default)]
    pub spans: Vec<RustcSpan>,
}

#[derive(Deserialize)]
pub(super) struct RustcCode {
    pub code: String,
}

#[derive(Deserialize)]
pub(super) struct RustcSpan {
    pub file_name: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub is_primary: bool,
}
