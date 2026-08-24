use std::process::{Command, Stdio};

use policy_core::{
    AnalysisError, AnalysisInput, CodebaseFacts, SemanticFindingFact, SemanticFindingKind,
};

use super::{
    SEMANTIC_TOOLCHAIN,
    types::{CargoMessage, RawLocation, push_unique, source_location},
};

pub(super) fn collect(
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
) -> Result<(), AnalysisError> {
    let output = run_rustc(input)?;
    let (dead_count, other_errors) = parse_messages(input, facts, &output.stdout);
    if output.status.success() || (other_errors.is_empty() && dead_count > 0) {
        return Ok(());
    }
    let detail = if other_errors.is_empty() {
        String::from_utf8_lossy(&output.stderr).trim().to_owned()
    } else {
        other_errors.join("; ")
    };
    Err(AnalysisError::new(format!(
        "rustc dead-code analysis could not complete: {detail}"
    )))
}

fn run_rustc(input: &AnalysisInput) -> Result<std::process::Output, AnalysisError> {
    rustc_command(input).output().map_err(|error| {
        AnalysisError::new(format!("could not run rustc dead-code check: {error}"))
    })
}

fn rustc_command(input: &AnalysisInput) -> Command {
    let mut command = Command::new("rustup");
    command
        .args(["run", SEMANTIC_TOOLCHAIN, "cargo", "check"])
        .current_dir(&input.workspace_root)
        .arg("--manifest-path")
        .arg(input.workspace_root.join("Cargo.toml"));
    crate::artifacts::configure_cargo(&mut command, input.workspace_root.as_std_path());
    command
        .args([
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--message-format=json",
            "--color=never",
        ])
        .stderr(Stdio::piped());
    if super::offline() {
        command.env("CARGO_NET_OFFLINE", "true");
    }
    command
}

fn parse_messages(
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
    output: &[u8],
) -> (usize, Vec<String>) {
    let mut dead_count = 0;
    let mut other_errors = Vec::new();
    for line in output.split(|byte| *byte == b'\n') {
        let Ok(message) = serde_json::from_slice::<CargoMessage>(line) else {
            continue;
        };
        let Some(diagnostic) = message.message else {
            continue;
        };
        let is_dead = diagnostic
            .code
            .as_ref()
            .is_some_and(|code| code.code == "dead_code");
        if diagnostic.level == "error" && !is_dead {
            other_errors.push(diagnostic.message.clone());
        }
        if is_dead {
            dead_count += append_diagnostic(input, facts, diagnostic);
        }
    }
    (dead_count, other_errors)
}

fn append_diagnostic(
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
    diagnostic: super::types::RustcDiagnostic,
) -> usize {
    let mut count = 0;
    for location in diagnostic.spans.into_iter().filter(|span| span.is_primary) {
        let raw = RawLocation::from(location);
        let Some((path, span)) = source_location(input, &raw) else {
            continue;
        };
        push_unique(
            facts,
            SemanticFindingFact {
                kind: SemanticFindingKind::PrivateDeadCode,
                item: diagnostic.message.clone(),
                item_kind: None,
                path,
                span,
            },
        );
        count += 1;
    }
    count
}

#[cfg(test)]
#[path = "tests/private_dead.rs"]
mod tests;
