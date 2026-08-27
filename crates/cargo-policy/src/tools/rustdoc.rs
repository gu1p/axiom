use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use policy_core::{AnalysisError, AnalysisInput, ClippyConfig};

use super::clippy::{feature_arguments, profile};
use super::{RunMode, ToolReport, cargo, offline};

const TOOL_NAME: &str = "rustdoc";

pub fn run(
    input: &AnalysisInput,
    config: &ClippyConfig,
    mode: RunMode,
    target_dir: &Path,
) -> Result<ToolReport, AnalysisError> {
    let mut command = command(input, config, mode.fail_fast, target_dir);
    cargo::execute(TOOL_NAME, input, &mut command, mode)
}

fn command(
    input: &AnalysisInput,
    config: &ClippyConfig,
    fail_fast: bool,
    target_dir: &Path,
) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(&input.workspace_root)
        .arg("doc")
        .arg("--manifest-path")
        .arg(input.workspace_root.join("Cargo.toml"));
    crate::artifacts::configure_cargo(&mut command, target_dir);
    command.args(["--workspace", "--locked", "--no-deps"]);
    if fail_fast {
        command.args(["--jobs", "1"]);
    } else {
        command.arg("--keep-going");
    }
    command.args(["--quiet", "--message-format=json", "--color=never"]);
    feature_arguments(&mut command, config);
    command.env("RUSTDOCFLAGS", rustdoc_flags(config));
    if offline() {
        command.env("CARGO_NET_OFFLINE", "true");
    }
    command
}

fn rustdoc_flags(config: &ClippyConfig) -> OsString {
    let mut flags = env::var_os("RUSTDOCFLAGS").unwrap_or_default();
    for argument in profile::rustdoc_arguments(config) {
        if !flags.is_empty() {
            flags.push(" ");
        }
        flags.push(argument);
    }
    flags
}

#[cfg(test)]
#[path = "tests/rustdoc.rs"]
mod tests;
