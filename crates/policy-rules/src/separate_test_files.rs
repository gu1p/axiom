use globset::{Glob, GlobSet, GlobSetBuilder};
use policy_core::{
    CodebaseFacts, Diagnostic, Level, Rule, RuleClass, RuleFactory, RuleMetadata, SourceKind,
    TestCodeFact, TestCodeKind,
};
use serde::Deserialize;
use toml::Table;

use crate::SEPARATE_TEST_FILES;

pub struct SeparateTestFilesFactory;

impl RuleFactory for SeparateTestFilesFactory {
    fn id(&self) -> &'static str {
        SEPARATE_TEST_FILES
    }

    fn create(&self, table: &Table) -> Result<Box<dyn Rule>, String> {
        let config = SeparateTestFilesConfig::parse(self.id(), table)?;
        let test_files = build_globs(self.id(), &config.test_file_patterns)?;
        Ok(Box::new(SeparateTestFiles {
            level: config.level,
            test_files,
        }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeparateTestFilesConfig {
    level: Level,
    #[serde(default = "default_test_file_patterns")]
    test_file_patterns: Vec<String>,
}

impl SeparateTestFilesConfig {
    fn parse(id: &str, table: &Table) -> Result<Self, String> {
        let config: Self = toml::Value::Table(table.clone())
            .try_into()
            .map_err(|error| format!("invalid configuration for `{id}`: {error}"))?;
        if config.test_file_patterns.is_empty() {
            return Err(format!(
                "invalid configuration for `{id}`: test_file_patterns must not be empty"
            ));
        }
        Ok(config)
    }
}

struct SeparateTestFiles {
    level: Level,
    test_files: GlobSet,
}

impl Rule for SeparateTestFiles {
    fn metadata(&self) -> RuleMetadata {
        RuleMetadata {
            id: SEPARATE_TEST_FILES,
            class: RuleClass::Invariant,
            description: "Test implementations must live in dedicated Rust test files",
        }
    }

    fn level(&self) -> Level {
        self.level
    }

    fn evaluate(&self, facts: &CodebaseFacts, diagnostics: &mut Vec<Diagnostic>) {
        for file in &facts.files {
            if file.source.kind == SourceKind::Test {
                continue;
            }
            let path = file.source.relative_path.as_str().replace('\\', "/");
            if self.test_files.is_match(&path) {
                continue;
            }
            diagnostics.extend(file.test_code.iter().map(|fact| Diagnostic {
                rule_id: SEPARATE_TEST_FILES.to_owned(),
                class: RuleClass::Invariant,
                level: self.level,
                message: message(fact),
                help: "move the test implementation into a dedicated test file".to_owned(),
                path: file.source.relative_path.clone(),
                span: fact.span,
                observed: None,
                limit: None,
                unit: None,
            }));
        }
    }
}

fn message(fact: &TestCodeFact) -> String {
    let name = fact
        .name
        .as_deref()
        .map_or_else(String::new, |name| format!(" `{name}`"));
    match fact.kind {
        TestCodeKind::TestFunction => {
            format!("test function{name} is declared in a production file")
        }
        TestCodeKind::InlineTestModule => {
            format!("test module{name} is implemented in a production file")
        }
        TestCodeKind::TestOnlyItem => {
            format!("test-only item{name} is declared in a production file")
        }
    }
}

fn build_globs(id: &str, patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|error| {
            format!("invalid configuration for `{id}`: invalid test glob `{pattern}`: {error}")
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|error| format!("invalid configuration for `{id}`: {error}"))
}

fn default_test_file_patterns() -> Vec<String> {
    [
        "tests.rs",
        "*_test.rs",
        "*_tests.rs",
        "tests/**/*.rs",
        "**/tests.rs",
        "**/*_test.rs",
        "**/*_tests.rs",
        "**/tests/**/*.rs",
    ]
    .map(str::to_owned)
    .to_vec()
}
