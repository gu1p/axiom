use camino::Utf8Path;
use policy_core::{AnalysisError, Diagnostic, Level, RuleClass, SourceSpan};
use serde::Serialize;

use crate::tools::{ToolDiagnostic, ToolReport};

use super::configuration;

#[derive(Serialize)]
struct JsonOutput {
    schema_version: u32,
    outcome: &'static str,
    diagnostics: Vec<JsonDiagnostic>,
    summary: Summary,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    kind: &'static str,
    tool: Option<&'static str>,
    rule_id: Option<String>,
    class: Option<RuleClass>,
    level: &'static str,
    message: String,
    help: Option<String>,
    path: Option<camino::Utf8PathBuf>,
    span: Option<SourceSpan>,
    observed: Option<u32>,
    limit: Option<u32>,
    configuration: Option<configuration::JsonHint>,
}

#[derive(Serialize)]
struct Summary {
    errors: usize,
    warnings: usize,
}

pub fn check(
    diagnostics: &[Diagnostic],
    tools: &[ToolReport],
    config_path: &Utf8Path,
    stopped: bool,
) {
    let errors = diagnostics
        .iter()
        .filter(|item| item.level == Level::Deny)
        .count()
        + tools
            .iter()
            .flat_map(|report| &report.diagnostics)
            .filter(|item| item.level == Level::Deny)
            .count();
    let warnings = diagnostics
        .iter()
        .filter(|item| item.level == Level::Warn)
        .count()
        + tools
            .iter()
            .flat_map(|report| &report.diagnostics)
            .filter(|item| item.level == Level::Warn)
            .count();
    let json_diagnostics = diagnostics
        .iter()
        .map(|diagnostic| policy_diagnostic(diagnostic, config_path))
        .chain(
            tools
                .iter()
                .flat_map(|report| &report.diagnostics)
                .map(|diagnostic| tool_diagnostic(diagnostic, config_path)),
        )
        .collect();
    let output = JsonOutput {
        schema_version: 1,
        outcome: if errors == 0 && !stopped {
            "passed"
        } else {
            "violations"
        },
        diagnostics: json_diagnostics,
        summary: Summary { errors, warnings },
    };
    print_json(&output);
}

pub fn operational(errors: &[AnalysisError]) {
    let output = JsonOutput {
        schema_version: 1,
        outcome: "error",
        diagnostics: errors.iter().map(JsonDiagnostic::from).collect(),
        summary: Summary {
            errors: errors.len(),
            warnings: 0,
        },
    };
    print_json(&output);
}

impl From<&Diagnostic> for JsonDiagnostic {
    fn from(value: &Diagnostic) -> Self {
        Self {
            kind: "policy",
            tool: None,
            rule_id: Some(value.rule_id.clone()),
            class: Some(value.class),
            level: if value.level == Level::Warn {
                "warning"
            } else {
                "error"
            },
            message: value.message.clone(),
            help: Some(value.help.clone()),
            path: Some(value.path.clone()),
            span: Some(value.span),
            observed: value.observed,
            limit: value.limit,
            configuration: None,
        }
    }
}

impl From<&AnalysisError> for JsonDiagnostic {
    fn from(value: &AnalysisError) -> Self {
        Self {
            kind: "operational",
            tool: None,
            rule_id: None,
            class: None,
            level: "error",
            message: value.message.clone(),
            help: None,
            path: value.path.clone(),
            span: value.span,
            observed: None,
            limit: None,
            configuration: None,
        }
    }
}

impl From<&ToolDiagnostic> for JsonDiagnostic {
    fn from(value: &ToolDiagnostic) -> Self {
        Self {
            kind: "tool",
            tool: Some(value.tool),
            rule_id: Some(value.rule_id.clone()),
            class: None,
            level: if value.level == Level::Warn {
                "warning"
            } else {
                "error"
            },
            message: value.message.clone(),
            help: value.help.clone(),
            path: value.path.clone(),
            span: value.span,
            observed: None,
            limit: None,
            configuration: None,
        }
    }
}

fn policy_diagnostic(value: &Diagnostic, path: &Utf8Path) -> JsonDiagnostic {
    let mut diagnostic = JsonDiagnostic::from(value);
    diagnostic.configuration = Some(configuration::json(path, configuration::policy(value)));
    diagnostic
}

fn tool_diagnostic(value: &ToolDiagnostic, path: &Utf8Path) -> JsonDiagnostic {
    let mut diagnostic = JsonDiagnostic::from(value);
    diagnostic.configuration =
        configuration::tool(value).map(|hint| configuration::json(path, hint));
    diagnostic
}

fn print_json(output: &JsonOutput) {
    use std::io::Write as _;

    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, output).expect("writing JSON to stdout");
    writeln!(stdout).expect("writing JSON terminator to stdout");
}
