use core::time::Duration;

use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, Diagnostic, Level};

use crate::args::{CheckOptions, OutputFormat};
use crate::report;
use crate::tools::{RunEvent, ToolReport};

pub(super) fn policies(
    result: &Result<Vec<Diagnostic>, Vec<AnalysisError>>,
    elapsed: Duration,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) {
    if options.format != OutputFormat::Human {
        return;
    }
    match result {
        Ok(items) => {
            report::progress::finished("policies", elapsed);
            report::human::write_policy_diagnostics(items, input, config_path, options.color);
        }
        Err(_) => report::progress::failed("policies"),
    }
}

pub(super) fn tool(
    event: RunEvent,
    active_tool: &mut Option<&'static str>,
    tool_reports: &mut Vec<ToolReport>,
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
) {
    match event {
        RunEvent::Started(name) => {
            *active_tool = Some(name);
            if options.format == OutputFormat::Human {
                report::progress::started(name, options.color);
            }
        }
        RunEvent::Finished(mut tool_report, elapsed) => {
            *active_tool = None;
            if options.ignore_warnings {
                tool_report
                    .diagnostics
                    .retain(|item| item.level != Level::Warn);
            }
            if options.format == OutputFormat::Human {
                report::progress::finished(tool_report.name, elapsed);
                report::human::write_tool_report(&tool_report, input, config_path, options.color);
            }
            tool_reports.push(tool_report);
        }
    }
}
