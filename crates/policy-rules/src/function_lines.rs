use policy_core::{CodebaseFacts, Diagnostic, Level, Rule, RuleClass, RuleFactory, RuleMetadata};
use toml::Table;

use crate::{FUNCTION_MAX_LINES, limit::LimitConfig};

pub struct FunctionMaxLinesFactory;

impl RuleFactory for FunctionMaxLinesFactory {
    fn id(&self) -> &'static str {
        FUNCTION_MAX_LINES
    }

    fn create(&self, table: &Table) -> Result<Box<dyn Rule>, String> {
        let config = LimitConfig::parse(self.id(), table)?;
        Ok(Box::new(FunctionMaxLines(config)))
    }
}

struct FunctionMaxLines(LimitConfig);

impl Rule for FunctionMaxLines {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: FUNCTION_MAX_LINES,
            class: RuleClass::Budget,
            description: "Rust functions must stay within the configured physical line budget",
        }
    }

    fn level(&self) -> Level {
        self.0.level
    }

    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>) {
        for file in &facts.files {
            for function in &file.functions {
                if function.line_count <= self.0.limit {
                    continue;
                }
                diagnostics.push(Diagnostic {
                    rule_id: FUNCTION_MAX_LINES.to_owned(),
                    class: RuleClass::Budget,
                    level: self.0.level,
                    message: format!(
                        "function `{}` has {} physical lines",
                        function.name, function.line_count
                    ),
                    help: "extract cohesive work into smaller functions".to_owned(),
                    path: file.source.relative_path.clone(),
                    span: function.name_span,
                    observed: Some(function.line_count),
                    limit: Some(self.0.limit),
                });
            }
        }
    }
}
