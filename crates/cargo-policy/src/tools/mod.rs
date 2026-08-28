mod cargo;
mod clippy;
mod process;
mod rustdoc;

pub(crate) use process::{configure_group, terminate_group};

use core::time::Duration;
use std::env;
use std::path::Path;
use std::time::Instant;

use camino::Utf8PathBuf;
use policy_core::{AnalysisError, AnalysisInput, Level, SourceSpan, ToolConfig};

use crate::check::selection::{Family, Selection};

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

#[derive(Clone, Copy)]
pub(crate) struct RunMode {
    pub fail_fast: bool,
    pub ignore_warnings: bool,
}

impl RunMode {
    const COMPLETE: Self = Self {
        fail_fast: false,
        ignore_warnings: false,
    };

    pub(super) const fn fail_fast(ignore_warnings: bool) -> Self {
        Self {
            fail_fast: true,
            ignore_warnings,
        }
    }
}

pub fn run_each(
    input: &AnalysisInput,
    config: &ToolConfig,
    selection: Selection,
    target_dir: &Path,
    mut emit: impl FnMut(RunEvent),
) -> Result<(), AnalysisError> {
    for family in [Family::Clippy, Family::Rustdoc] {
        if !enabled(config, selection, family) {
            continue;
        }
        let name = name(family);
        emit(RunEvent::Started(name));
        let started = Instant::now();
        let report = run(input, config, family, RunMode::COMPLETE, target_dir)?;
        emit(RunEvent::Finished(report, started.elapsed()));
    }
    Ok(())
}

pub(super) fn any_enabled(config: &ToolConfig, selection: Selection) -> bool {
    [Family::Clippy, Family::Rustdoc]
        .into_iter()
        .any(|family| enabled(config, selection, family))
}

pub(super) fn enabled(config: &ToolConfig, selection: Selection, family: Family) -> bool {
    config.clippy.enabled
        && selection.includes(family)
        && (family != Family::Rustdoc || config.clippy.check_docs)
}

pub(super) fn run(
    input: &AnalysisInput,
    config: &ToolConfig,
    family: Family,
    mode: RunMode,
    target_dir: &Path,
) -> Result<ToolReport, AnalysisError> {
    match family {
        Family::Clippy => clippy::run(input, &config.clippy, mode, target_dir),
        Family::Rustdoc => rustdoc::run(input, &config.clippy, mode, target_dir),
        _ => unreachable!("native policy families are not external tools"),
    }
}

pub(super) const fn name(family: Family) -> &'static str {
    match family {
        Family::Clippy => "clippy",
        Family::Rustdoc => "rustdoc",
        _ => unreachable!(),
    }
}

fn offline() -> bool {
    env::var("AXIOM_OFFLINE")
        .is_ok_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}
