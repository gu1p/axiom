pub mod human;
pub mod json;

use policy_core::{AnalysisInput, Diagnostic};

use crate::args::{CheckOptions, OutputFormat};

pub fn policies(options: &CheckOptions, input: &AnalysisInput, diagnostics: &[Diagnostic]) {
    match options.format {
        OutputFormat::Human => human::policies(diagnostics, input, options.color),
        OutputFormat::Json => json::policies(diagnostics),
    }
}
