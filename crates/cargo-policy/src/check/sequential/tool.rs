use std::path::Path;
use std::time::Instant;

use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, Level, PolicyConfig};

use super::super::{
    Analysis,
    selection::{Family, Selection},
};
use crate::args::{CheckOptions, OutputFormat};
use crate::report;
use crate::tools::{self, ToolReport};

pub(super) enum Step {
    Continue(Vec<ToolReport>),
    Stopped(Analysis),
}

pub(super) fn run_selected(
    config: &PolicyConfig,
    input: &AnalysisInput,
    options: &CheckOptions,
    selection: Selection,
    config_path: &Utf8Path,
    artifacts: &crate::artifacts::RunArtifacts,
) -> Result<Step, Vec<AnalysisError>> {
    let cargo_target_dir = artifacts.cargo_target_dir();
    let mut tool_reports = Vec::new();
    for family in [Family::Clippy, Family::Rustdoc] {
        if !tools::enabled(&config.tools, selection, family) {
            continue;
        }
        let report = run(
            family,
            config,
            input,
            options,
            config_path,
            &cargo_target_dir,
        )?;
        let stopped = !report.diagnostics.is_empty();
        tool_reports.push(report);
        if stopped {
            return Ok(Step::Stopped(Analysis {
                diagnostics: Vec::new(),
                tool_reports,
                stopped: true,
            }));
        }
    }
    Ok(Step::Continue(tool_reports))
}

fn run(
    family: Family,
    config: &PolicyConfig,
    input: &AnalysisInput,
    options: &CheckOptions,
    config_path: &Utf8Path,
    cargo_target_dir: &Path,
) -> Result<ToolReport, Vec<AnalysisError>> {
    let name = tools::name(family);
    let human = options.format == OutputFormat::Human;
    if human {
        report::progress::started(name, options.color);
    }
    let started = Instant::now();
    let mode = tools::RunMode::fail_fast(options.ignore_warnings);
    let mut tool_report = match tools::run(input, &config.tools, family, mode, cargo_target_dir) {
        Ok(report) => report,
        Err(error) => {
            if human {
                report::progress::failed(name);
            }
            return Err(vec![error]);
        }
    };
    if options.ignore_warnings {
        tool_report
            .diagnostics
            .retain(|item| item.level != Level::Warn);
    }
    tool_report.diagnostics.truncate(1);
    if human {
        report::progress::finished(name, started.elapsed());
        report::human::write_tool_report(&tool_report, input, config_path, options.color);
    }
    Ok(tool_report)
}
