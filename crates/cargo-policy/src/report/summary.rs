use policy_core::{AnalysisInput, Diagnostic, Level};

use crate::tools::ToolReport;

use super::output::stderr_line;

pub(crate) fn write(
    diagnostics: &[Diagnostic],
    tools: &[ToolReport],
    input: &AnalysisInput,
    stopped: bool,
) {
    let (denied, warned) = counts(diagnostics, tools);
    if stopped {
        stderr_line(format_args!(
            "axiom check stopped after first finding ({denied} error(s) and {warned} warning(s))"
        ));
        return;
    }
    if denied == 0 && warned == 0 {
        passed(input, tools);
    } else {
        stderr_line(format_args!(
            "axiom check found {denied} error(s) and {warned} warning(s)"
        ));
    }
}

fn counts(diagnostics: &[Diagnostic], tools: &[ToolReport]) -> (usize, usize) {
    let denied = count(Level::Deny, diagnostics, tools);
    let warned = count(Level::Warn, diagnostics, tools);
    (denied, warned)
}

fn count(level: Level, diagnostics: &[Diagnostic], tools: &[ToolReport]) -> usize {
    diagnostics
        .iter()
        .filter(|item| item.level == level)
        .count()
        + tools
            .iter()
            .flat_map(|report| &report.diagnostics)
            .filter(|item| item.level == level)
            .count()
}

fn passed(input: &AnalysisInput, tools: &[ToolReport]) {
    let tools = tools
        .iter()
        .map(|report| report.name)
        .collect::<Vec<_>>()
        .join(", ");
    if tools.is_empty() {
        stderr_line(format_args!(
            "axiom check passed ({} Rust files)",
            input.sources.len()
        ));
    } else {
        stderr_line(format_args!(
            "axiom check passed ({} Rust files; {tools} passed)",
            input.sources.len()
        ));
    }
}
