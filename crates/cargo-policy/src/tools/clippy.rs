pub(super) mod profile;

use std::process::Command;

use policy_core::{AnalysisError, AnalysisInput, ClippyConfig};

use super::{RunMode, ToolReport, cargo, offline};

const TOOL_NAME: &str = "clippy";

pub fn run(
    input: &AnalysisInput,
    config: &ClippyConfig,
    mode: RunMode,
) -> Result<ToolReport, AnalysisError> {
    let mut command = command(input, config, mode.fail_fast);
    cargo::execute(TOOL_NAME, input, &mut command, mode).map_err(|error| {
        if error.message.contains("could not run") {
            AnalysisError::new(format!(
                "{}; install Clippy with `rustup component add clippy`",
                error.message
            ))
        } else {
            error
        }
    })
}

fn command(input: &AnalysisInput, config: &ClippyConfig, fail_fast: bool) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(&input.workspace_root)
        .arg("clippy")
        .arg("--manifest-path")
        .arg(input.workspace_root.join("Cargo.toml"));
    crate::artifacts::configure_cargo(&mut command, input.workspace_root.as_std_path());
    command.args(["--workspace", "--locked", "--no-deps"]);
    if fail_fast {
        command.args(["--jobs", "1"]);
    } else {
        command.arg("--keep-going");
    }
    command.args(["--quiet", "--message-format=json", "--color=never"]);
    if config.checks_all_targets() {
        command.arg("--all-targets");
    }
    feature_arguments(&mut command, config);
    let lints = profile::compiler_arguments(config);
    if !lints.is_empty() {
        command.arg("--").args(lints);
    }
    if offline() {
        command.env("CARGO_NET_OFFLINE", "true");
    }
    command
}

pub(super) fn feature_arguments(command: &mut Command, config: &ClippyConfig) {
    if config.checks_all_features() {
        command.arg("--all-features");
    } else if let Some(features) = config.selected_features() {
        command.arg("--features").arg(features.join(","));
    }
    if config.no_default_features {
        command.arg("--no-default-features");
    }
}

#[cfg(test)]
#[path = "tests/clippy.rs"]
mod tests;
