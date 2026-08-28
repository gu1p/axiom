//! Shared facts, configuration, diagnostics, and execution primitives for Axiom.

mod config;
mod diagnostic;
mod engine;
mod facts;
mod lines;

pub use config::{
    ClippyConfig, ClippyFeatureMode, ClippyFeatureSelection, ClippyLintProfile,
    ClippyTargetCoverage, ClippyWarningPolicy, PolicyConfig, SourceConfig, ToolConfig,
};
pub use diagnostic::{
    Diagnostic, Level, MetricUnit, Position, RuleClass, RuleMetadata, RuleScope, SourceSpan,
};
pub use engine::{Engine, FactProvider, Rule, RuleFactory, RuleRegistry};
pub use facts::{
    AnalysisInput, CodebaseFacts, FunctionFact, RustEdition, SemanticFindingFact,
    SemanticFindingKind, SourceFileFact, SourceKind, SourceUnit, TestCodeFact, TestCodeKind,
};
pub use lines::LineIndex;

use camino::Utf8PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct AnalysisError {
    pub message: String,
    pub path: Option<Utf8PathBuf>,
    pub span: Option<SourceSpan>,
}

impl AnalysisError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            span: None,
        }
    }

    #[must_use]
    pub fn at(mut self, path: Utf8PathBuf, span: Option<SourceSpan>) -> Self {
        self.path = Some(path);
        self.span = span;
        self
    }
}
