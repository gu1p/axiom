mod cargo;
mod clippy;
mod rustdoc;

use std::env;

use camino::Utf8PathBuf;
use policy_core::{AnalysisError, AnalysisInput, Level, SourceSpan, ToolConfig};

#[derive(Debug)]
pub struct ToolDiagnostic {
    pub tool: &'static str,
    pub rule_id: String,
    pub level: Level,
    pub message: String,
    pub help: Option<String>,
    pub path: Option<Utf8PathBuf>,
    pub span: Option<SourceSpan>,
    pub rendered: Option<String>,
}

#[derive(Debug)]
pub struct ToolReport {
    pub name: &'static str,
    pub diagnostics: Vec<ToolDiagnostic>,
}

pub fn run(input: &AnalysisInput, config: &ToolConfig) -> Result<Vec<ToolReport>, AnalysisError> {
    let mut reports = Vec::new();
    if config.clippy.enabled {
        reports.push(clippy::run(input, &config.clippy)?);
        if config.clippy.check_docs {
            reports.push(rustdoc::run(input, &config.clippy)?);
        }
    }
    Ok(reports)
}

fn offline() -> bool {
    env::var("AXIOM_OFFLINE")
        .is_ok_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}
