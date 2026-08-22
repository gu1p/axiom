use policy_core::{CodebaseFacts, Diagnostic, Level, Rule, RuleClass, RuleFactory, RuleMetadata};
use toml::Table;

use crate::{FILE_MAX_LINES, limit::LimitConfig};

pub struct FileMaxLinesFactory;

impl RuleFactory for FileMaxLinesFactory {
    fn id(&self) -> &'static str {
        FILE_MAX_LINES
    }

    fn create(&self, table: &Table) -> Result<Box<dyn Rule>, String> {
        let config = LimitConfig::parse(self.id(), table)?;
        Ok(Box::new(FileMaxLines(config)))
    }
}

struct FileMaxLines(LimitConfig);

impl Rule for FileMaxLines {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: FILE_MAX_LINES,
            class: RuleClass::Budget,
            description: "Rust files must stay within the configured physical line budget",
        }
    }

    fn level(&self) -> Level {
        self.0.level
    }

    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>) {
        for file in &facts.files {
            if file.line_count <= self.0.limit {
                continue;
            }
            let end = usize::from(!file.source.text.is_empty());
            diagnostics.push(Diagnostic {
                rule_id: FILE_MAX_LINES.to_owned(),
                class: RuleClass::Budget,
                level: self.0.level,
                message: format!("file has {} physical lines", file.line_count),
                help: "split the file into focused modules".to_owned(),
                path: file.source.relative_path.clone(),
                span: file.source.lines.span(&file.source.text, 0, end),
                observed: file.line_count,
                limit: self.0.limit,
            });
        }
    }
}
