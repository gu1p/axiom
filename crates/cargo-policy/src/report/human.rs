use std::io::{self, IsTerminal as _};

use policy_core::{AnalysisError, AnalysisInput, Diagnostic, Level, SourceSpan, SourceUnit};

use crate::args::ColorChoice;
use crate::tools::{ToolDiagnostic, ToolReport};

use super::output::{stderr_line, stderr_write};

pub fn check(
    diagnostics: &[Diagnostic],
    tools: &[ToolReport],
    input: &AnalysisInput,
    choice: ColorChoice,
) {
    let color = color_enabled(choice);
    policy_diagnostics(diagnostics, input, color);
    for report in tools {
        for diagnostic in &report.diagnostics {
            tool_diagnostic(diagnostic, input, color);
        }
    }
    let (denied, warned) = diagnostic_counts(diagnostics, tools);
    summary(input, tools, denied, warned);
}

fn policy_diagnostics(diagnostics: &[Diagnostic], input: &AnalysisInput, color: bool) {
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
        stderr_line(format_args!("  = help: {}\n", diagnostic.help));
    }
}

fn diagnostic_counts(diagnostics: &[Diagnostic], tools: &[ToolReport]) -> (usize, usize) {
    let denied = diagnostics
        .iter()
        .filter(|item| item.level == Level::Deny)
        .count()
        + tools
            .iter()
            .flat_map(|report| &report.diagnostics)
            .filter(|item| item.level == Level::Deny)
            .count();
    let warned = diagnostics
        .iter()
        .filter(|item| item.level == Level::Warn)
        .count()
        + tools
            .iter()
            .flat_map(|report| &report.diagnostics)
            .filter(|item| item.level == Level::Warn)
            .count();
    (denied, warned)
}

fn summary(input: &AnalysisInput, tools: &[ToolReport], denied: usize, warned: usize) {
    if denied == 0 && warned == 0 {
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
    } else {
        stderr_line(format_args!(
            "axiom check found {denied} error(s) and {warned} warning(s)"
        ));
    }
}

fn tool_diagnostic(diagnostic: &ToolDiagnostic, input: &AnalysisInput, color: bool) {
    if let Some(rendered) = &diagnostic.rendered {
        stderr_write(format_args!("{rendered}"));
        if !rendered.ends_with('\n') {
            stderr_line(format_args!(""));
        }
        return;
    }
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
    stderr_line(format_args!(""));
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

fn color_enabled(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => io::stderr().is_terminal(),
    }
}

fn severity_color(severity: &str) -> &'static str {
    if severity == "warning" { "33" } else { "31" }
}

fn paint(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("\u{1b}[{code}m{text}\u{1b}[0m")
    } else {
        text.to_owned()
    }
}
