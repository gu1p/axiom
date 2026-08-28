use std::collections::BTreeMap;

use policy_core::{
    CodebaseFacts, Diagnostic, Level, MetricUnit, Rule, RuleClass, RuleFactory, RuleMetadata,
    RuleScope, SourceFileFact, SourceKind,
};
use toml::Table;

use super::{DIRECTORY_MAX_FILES, DIRECTORY_MAX_LINES, limit::LimitConfig};

pub(super) struct DirectoryMaxFilesFactory;

impl RuleFactory for DirectoryMaxFilesFactory {
    fn id(&self) -> &'static str {
        DIRECTORY_MAX_FILES
    }

    fn create(&self, table: &Table) -> Result<Box<dyn Rule>, String> {
        Ok(Box::new(DirectoryLimit {
            config: LimitConfig::parse(self.id(), table)?,
            metric: DirectoryMetric::Files,
        }))
    }
}

pub(super) struct DirectoryMaxLinesFactory;

impl RuleFactory for DirectoryMaxLinesFactory {
    fn id(&self) -> &'static str {
        DIRECTORY_MAX_LINES
    }

    fn create(&self, table: &Table) -> Result<Box<dyn Rule>, String> {
        Ok(Box::new(DirectoryLimit {
            config: LimitConfig::parse(self.id(), table)?,
            metric: DirectoryMetric::CodeLines,
        }))
    }
}

struct DirectoryLimit {
    config: LimitConfig,
    metric: DirectoryMetric,
}

impl Rule for DirectoryLimit {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: self.metric.rule_id(),
            class: RuleClass::Budget,
            description: self.metric.description(),
        }
    }

    fn level(&self) -> Level {
        self.config.level
    }

    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>) {
        self.evaluate_directories(facts, RuleScope::All, diagnostics);
    }

    fn evaluate_scoped(
        &self,
        facts: &CodebaseFacts,
        scope: RuleScope,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        self.evaluate_directories(facts, scope, diagnostics);
    }
}

impl DirectoryLimit {
    fn evaluate_directories(
        &self,
        facts: &CodebaseFacts,
        scope: RuleScope,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut directories = BTreeMap::<&str, Vec<&SourceFileFact>>::new();
        for file in facts
            .files
            .iter()
            .filter(|file| matches_scope(file.source.kind, scope))
        {
            let directory = file
                .source
                .relative_path
                .parent()
                .map_or("", |path| path.as_str());
            directories.entry(directory).or_default().push(file);
        }

        for (directory, mut files) in directories {
            files.sort_by(|left, right| left.source.relative_path.cmp(&right.source.relative_path));
            let observed = files.iter().fold(0_u32, |total, file| {
                total.saturating_add(self.metric.contribution(file))
            });
            if observed <= self.config.limit {
                continue;
            }
            let Some(anchor) = threshold_crossing_file(&files, self.metric, self.config.limit)
            else {
                continue;
            };
            diagnostics.push(Diagnostic {
                rule_id: self.metric.rule_id().to_owned(),
                class: RuleClass::Budget,
                level: self.config.level,
                message: self.metric.message(display_directory(directory), observed),
                help: "split the directory into smaller cohesive subdomains".to_owned(),
                path: anchor.source.relative_path.clone(),
                span: anchor.source.lines.span(&anchor.source.text, 0, 0),
                observed: Some(observed),
                limit: Some(self.config.limit),
                unit: Some(self.metric.unit()),
            });
        }
    }
}

#[derive(Clone, Copy)]
enum DirectoryMetric {
    Files,
    CodeLines,
}

impl DirectoryMetric {
    const fn rule_id(self) -> &'static str {
        match self {
            Self::Files => DIRECTORY_MAX_FILES,
            Self::CodeLines => DIRECTORY_MAX_LINES,
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Files => "Directories must stay within the configured source-file budget",
            Self::CodeLines => "Directories must stay within the configured code-line budget",
        }
    }

    const fn unit(self) -> MetricUnit {
        match self {
            Self::Files => MetricUnit::Files,
            Self::CodeLines => MetricUnit::CodeLines,
        }
    }

    const fn contribution(self, file: &SourceFileFact) -> u32 {
        match self {
            Self::Files => 1,
            Self::CodeLines => file.code_line_count,
        }
    }

    fn message(self, directory: &str, observed: u32) -> String {
        match self {
            Self::Files => format!("directory `{directory}` contains {observed} source files"),
            Self::CodeLines => format!("directory `{directory}` contains {observed} code lines"),
        }
    }
}

fn matches_scope(kind: SourceKind, scope: RuleScope) -> bool {
    matches!(
        (kind, scope),
        (_, RuleScope::All)
            | (SourceKind::Production, RuleScope::Production)
            | (SourceKind::Test, RuleScope::Test)
    )
}

fn threshold_crossing_file<'a>(
    files: &[&'a SourceFileFact],
    metric: DirectoryMetric,
    limit: u32,
) -> Option<&'a SourceFileFact> {
    let mut subtotal = 0_u32;
    files.iter().copied().find(|file| {
        subtotal = subtotal.saturating_add(metric.contribution(file));
        subtotal > limit
    })
}

fn display_directory(directory: &str) -> &str {
    if directory.is_empty() { "." } else { directory }
}

#[cfg(test)]
#[path = "../tests/directory.rs"]
mod tests;
