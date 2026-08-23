use std::env;
use std::ffi::OsString;
use std::process::Command;

use policy_core::{AnalysisError, AnalysisInput, ClippyConfig};

use super::clippy::{feature_arguments, profile};
use super::{ToolReport, cargo, offline};

const TOOL_NAME: &str = "rustdoc";

pub fn run(input: &AnalysisInput, config: &ClippyConfig) -> Result<ToolReport, AnalysisError> {
    let mut command = command(input, config);
    cargo::execute(TOOL_NAME, input, &mut command)
}

fn command(input: &AnalysisInput, config: &ClippyConfig) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(&input.workspace_root)
        .arg("doc")
        .arg("--manifest-path")
        .arg(input.workspace_root.join("Cargo.toml"))
        .args([
            "--workspace",
            "--locked",
            "--no-deps",
            "--keep-going",
            "--quiet",
            "--message-format=json",
            "--color=never",
        ]);
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
