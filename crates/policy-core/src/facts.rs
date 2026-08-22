use std::sync::Arc;

use camino::Utf8PathBuf;

use crate::{LineIndex, RuleScope, SourceSpan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustEdition {
    Edition2015,
    Edition2018,
    Edition2021,
    Edition2024,
}

#[derive(Debug, Clone)]
pub struct SourceUnit {
    pub absolute_path: Utf8PathBuf,
    pub relative_path: Utf8PathBuf,
    pub edition: RustEdition,
    pub text: Arc<str>,
    pub lines: Arc<LineIndex>,
    pub kind: SourceKind,
}

impl SourceUnit {
    pub fn new(
        absolute_path: Utf8PathBuf,
        relative_path: Utf8PathBuf,
        edition: RustEdition,
        text: String,
    ) -> Self {
        let text: Arc<str> = text.into();
        let lines = Arc::new(LineIndex::new(&text));
        Self {
            absolute_path,
            relative_path,
            edition,
            text,
            lines,
            kind: SourceKind::Production,
        }
    }

    #[must_use]
    pub fn with_kind(mut self, kind: SourceKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Production,
    Test,
}

#[derive(Debug, Clone)]
pub struct AnalysisInput {
    pub workspace_root: Utf8PathBuf,
    pub sources: Vec<SourceUnit>,
}

#[derive(Debug, Clone)]
pub struct FunctionFact {
    pub name: String,
    pub span: SourceSpan,
    pub name_span: SourceSpan,
    pub line_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCodeKind {
    TestFunction,
    InlineTestModule,
    TestOnlyItem,
}

#[derive(Debug, Clone)]
pub struct TestCodeFact {
    pub kind: TestCodeKind,
    pub name: Option<String>,
    pub span: SourceSpan,
    pub item_span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct SourceFileFact {
    pub source: SourceUnit,
    pub line_count: u32,
    pub functions: Vec<FunctionFact>,
    pub test_code: Vec<TestCodeFact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticFindingKind {
    PrivateDeadCode,
    DeadPublic,
    TestOnly,
    UnnecessaryPublic,
    UnnecessaryRestrictedVisibility,
    UnnecessaryCrateVisibility,
}

#[derive(Debug, Clone)]
pub struct SemanticFindingFact {
    pub kind: SemanticFindingKind,
    pub item: String,
    pub item_kind: Option<String>,
    pub path: Utf8PathBuf,
    pub span: SourceSpan,
}

#[derive(Debug, Default)]
pub struct CodebaseFacts {
    pub files: Vec<SourceFileFact>,
    pub semantic_findings: Vec<SemanticFindingFact>,
}

impl CodebaseFacts {
    pub(super) fn matches_scope(
        &self,
        path: &camino::Utf8Path,
        offset: u32,
        scope: RuleScope,
    ) -> bool {
        if scope == RuleScope::All {
            return true;
        }
        let is_test = self.files.iter().any(|file| {
            file.source.relative_path == path
                && (file.source.kind == SourceKind::Test
                    || file
                        .test_code
                        .iter()
                        .any(|fact| contains(fact.item_span, offset)))
        });
        matches!(
            (scope, is_test),
            (RuleScope::Production, false) | (RuleScope::Test, true)
        )
    }
}

fn contains(span: SourceSpan, offset: u32) -> bool {
    span.byte_start <= offset && offset < span.byte_end
}
