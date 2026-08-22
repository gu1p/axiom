mod config;
mod diagnostic;
mod engine;
mod facts;
mod lines;
mod registry;

pub use config::{PolicyConfig, SourceConfig};
pub use diagnostic::{Diagnostic, Level, Position, RuleClass, RuleMetadata, SourceSpan};
pub use engine::{Engine, FactProvider, Rule};
pub use facts::{
    AnalysisInput, CodebaseFacts, FunctionFact, RustEdition, SourceFileFact, SourceUnit,
    TestCodeFact, TestCodeKind,
};
pub use lines::LineIndex;
pub use registry::{RuleFactory, RuleRegistry};

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
