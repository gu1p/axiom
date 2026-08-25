use std::time::Instant;

use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, Level, PolicyConfig};

use super::super::selection::Family;
use crate::args::{CheckOptions, OutputFormat};
use crate::report;
use crate::tools::{self, ToolReport};

pub(super) fn run(
    family: Family,
    config: &PolicyConfig,
    input: &AnalysisInput,
    options: &CheckOptions,
    config_path: &Utf8Path,
) -> Result<ToolReport, Vec<AnalysisError>> {
    let name = tools::name(family);
    let human = options.format == OutputFormat::Human;
    if human {
        report::progress::started(name);
    }
    let started = Instant::now();
    let mode = tools::RunMode::fail_fast(options.ignore_warnings);
    let mut tool_report = match tools::run(input, &config.tools, family, mode) {
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
