mod configuration;
mod formats;
mod output;
pub(super) mod progress;
pub(super) mod summary;

pub use formats::{human, json};

use camino::Utf8Path;
use policy_core::{AnalysisInput, Diagnostic};

use crate::args::{CheckOptions, OutputFormat};
use crate::tools::ToolReport;

pub fn check(
    options: &CheckOptions,
    input: &AnalysisInput,
    config_path: &Utf8Path,
    diagnostics: &[Diagnostic],
    tools: &[ToolReport],
    stopped: bool,
) {
    let config_path = config_path
        .strip_prefix(&input.workspace_root)
        .unwrap_or(config_path);
    match options.format {
        OutputFormat::Human => human::check(diagnostics, tools, input, config_path, options.color),
        OutputFormat::Json => json::check(diagnostics, tools, config_path, stopped),
    }
}
