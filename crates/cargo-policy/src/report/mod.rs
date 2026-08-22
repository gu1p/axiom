pub mod human;
pub mod json;

use policy_core::{AnalysisInput, Diagnostic};

use crate::args::{CheckOptions, OutputFormat};
use crate::tools::ToolReport;

pub fn check(
    options: &CheckOptions,
    input: &AnalysisInput,
    diagnostics: &[Diagnostic],
    tools: &[ToolReport],
) {
    match options.format {
        OutputFormat::Human => human::check(diagnostics, tools, input, options.color),
        OutputFormat::Json => json::check(diagnostics, tools),
    }
}
