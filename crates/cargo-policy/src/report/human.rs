use camino::Utf8Path;
use policy_core::{AnalysisError, AnalysisInput, Diagnostic, Level, SourceSpan, SourceUnit};

use crate::args::ColorChoice;
use crate::tools::{ToolDiagnostic, ToolReport};

use super::{
    configuration,
    output::{color_enabled, paint, severity_color, stderr_line, stderr_write},
};

pub fn check(
    diagnostics: &[Diagnostic],
    tools: &[ToolReport],
    input: &AnalysisInput,
    config_path: &Utf8Path,
    choice: ColorChoice,
) {
    let color = color_enabled(choice);
    policy_diagnostics(diagnostics, input, config_path, color);
    for report in tools {
        for diagnostic in &report.diagnostics {
            tool_diagnostic(diagnostic, input, config_path, color);
        }
    }
    super::summary::write(diagnostics, tools, input, false);
}

fn policy_diagnostics(
    diagnostics: &[Diagnostic],
    input: &AnalysisInput,
    config_path: &Utf8Path,
    color: bool,
) {
    for diagnostic in diagnostics {
        let severity = if diagnostic.level == Level::Warn {
            "warning"
        } else {
            "error"
        };
        let heading = paint(severity, severity_color(severity), color);
        stderr_line(format_args!(
            "{heading}[{}]: {}",
            diagnostic.rule_id, diagnostic.message
        ));
        location(&diagnostic.path, diagnostic.span, input, color);
        if let Some(limit) = diagnostic.limit {
            stderr_line(format_args!("  = limit: {limit} physical lines"));
        }
        stderr_line(format_args!("  = help: {}", diagnostic.help));
        configuration::write_human(config_path, &configuration::policy(diagnostic));
        stderr_line(format_args!(""));
    }
}

pub(crate) fn write_policy_diagnostics(
    diagnostics: &[Diagnostic],
    input: &AnalysisInput,
    config_path: &Utf8Path,
    choice: ColorChoice,
) {
    policy_diagnostics(diagnostics, input, config_path, color_enabled(choice));
}

fn tool_diagnostic(
    diagnostic: &ToolDiagnostic,
    input: &AnalysisInput,
    config_path: &Utf8Path,
    color: bool,
) {
    if let Some(rendered) = &diagnostic.rendered {
        stderr_write(format_args!("{rendered}"));
        if !rendered.ends_with('\n') {
            stderr_line(format_args!(""));
        }
    } else {
        let severity = if diagnostic.level == Level::Warn {
            "warning"
        } else {
            "error"
        };
        let heading = paint(severity, severity_color(severity), color);
        stderr_line(format_args!(
            "{heading}[{}::{}]: {}",
            diagnostic.tool, diagnostic.rule_id, diagnostic.message
        ));
        if let (Some(path), Some(span)) = (&diagnostic.path, diagnostic.span) {
            location(path, span, input, color);
        } else if let Some(path) = &diagnostic.path {
            stderr_line(format_args!(" --> {path}"));
        }
        if let Some(help) = &diagnostic.help {
            stderr_line(format_args!("  = help: {help}"));
        }
    }
    if let Some(hint) = configuration::tool(diagnostic) {
        configuration::write_human(config_path, &hint);
    }
    stderr_line(format_args!(""));
}

pub(crate) fn write_tool_report(
    report: &ToolReport,
    input: &AnalysisInput,
    config_path: &Utf8Path,
    choice: ColorChoice,
) {
    let color = color_enabled(choice);
    for diagnostic in &report.diagnostics {
        tool_diagnostic(diagnostic, input, config_path, color);
    }
}

pub fn operational(errors: &[AnalysisError], input: Option<&AnalysisInput>, choice: ColorChoice) {
    let color = color_enabled(choice);
    for error in errors {
        let heading = paint("error", "31", color);
        stderr_line(format_args!("{heading}[policy/tool]: {}", error.message));
        if let (Some(path), Some(span), Some(input)) = (&error.path, error.span, input) {
            location(path, span, input, color);
        } else if let Some(path) = &error.path {
            stderr_line(format_args!(" --> {path}"));
        }
        stderr_line(format_args!(""));
    }
    stderr_line(format_args!("axiom check could not complete"));
}

fn location(path: &camino::Utf8Path, span: SourceSpan, input: &AnalysisInput, color: bool) {
    stderr_line(format_args!(
        " --> {path}:{}:{}",
        span.start.line, span.start.column
    ));
    let Some(source) = input
        .sources
        .iter()
        .find(|source| source.relative_path == path)
    else {
        return;
    };
    snippet(source, span, color);
}

fn snippet(source: &SourceUnit, span: SourceSpan, color: bool) {
    let Some(line) = source.lines.line_text(&source.text, span.start.line) else {
        return;
    };
    let number = span.start.line;
    let gutter = number.to_string().len();
    let start = span.start.column.saturating_sub(1) as usize;
    let width = if span.end.line == span.start.line {
        span.end.column.saturating_sub(span.start.column).max(1) as usize
    } else {
        1
    };
    stderr_line(format_args!(" {:gutter$} |", "", gutter = gutter));
    stderr_line(format_args!(" {number:gutter$} | {line}"));
    let marker = format!("{}{}", " ".repeat(start), "^".repeat(width));
    stderr_line(format_args!(
        " {:gutter$} | {}",
        "",
        paint(&marker, "31", color),
        gutter = gutter
    ));
}
