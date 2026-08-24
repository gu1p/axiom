mod cargo;
mod clippy;
mod rustdoc;

use core::time::Duration;
use std::env;
use std::time::Instant;

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

pub enum RunEvent {
    Started(&'static str),
    Finished(ToolReport, Duration),
}

pub fn run_each(
    input: &AnalysisInput,
    config: &ToolConfig,
    mut emit: impl FnMut(RunEvent),
) -> Result<(), AnalysisError> {
    if config.clippy.enabled {
        emit(RunEvent::Started("clippy"));
        let started = Instant::now();
        emit(RunEvent::Finished(
            clippy::run(input, &config.clippy)?,
            started.elapsed(),
        ));
        if config.clippy.check_docs {
            emit(RunEvent::Started("rustdoc"));
            let started = Instant::now();
            emit(RunEvent::Finished(
                rustdoc::run(input, &config.clippy)?,
                started.elapsed(),
            ));
        }
    }
    Ok(())
}

fn offline() -> bool {
    env::var("AXIOM_OFFLINE")
        .is_ok_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}
