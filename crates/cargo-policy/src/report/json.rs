use policy_core::{AnalysisError, Diagnostic, Level, RuleClass, SourceSpan};
use serde::Serialize;

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
    rule_id: Option<String>,
    class: Option<RuleClass>,
    level: &'static str,
    message: String,
    help: Option<String>,
    path: Option<camino::Utf8PathBuf>,
    span: Option<SourceSpan>,
    observed: Option<u32>,
    limit: Option<u32>,
}

#[derive(Serialize)]
struct Summary {
    errors: usize,
    warnings: usize,
}

pub fn policies(diagnostics: &[Diagnostic]) {
    let errors = diagnostics
        .iter()
        .filter(|item| item.level == Level::Deny)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|item| item.level == Level::Warn)
        .count();
    let output = JsonOutput {
        schema_version: 1,
        outcome: if errors == 0 { "passed" } else { "violations" },
        diagnostics: diagnostics.iter().map(JsonDiagnostic::from).collect(),
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
            observed: Some(value.observed),
            limit: Some(value.limit),
        }
    }
}

impl From<&AnalysisError> for JsonDiagnostic {
    fn from(value: &AnalysisError) -> Self {
        Self {
            kind: "operational",
            rule_id: None,
            class: None,
            level: "error",
            message: value.message.clone(),
            help: None,
            path: value.path.clone(),
            span: value.span,
            observed: None,
            limit: None,
        }
    }
}

fn print_json(output: &JsonOutput) {
    serde_json::to_writer_pretty(std::io::stdout(), output).expect("writing JSON to stdout");
    println!();
}
