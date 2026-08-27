mod stream;

use std::{env, path::Path, process::Command};

use policy_core::{
    AnalysisError, AnalysisInput, CodebaseFacts, SemanticFindingFact, SemanticFindingKind,
};
use toml::Table;

use super::{
    SEMANTIC_TOOLCHAIN,
    config::semantic_config_file,
    types::{RawSemanticDiagnostic, SemanticReport, command_failure, push_unique, source_location},
};

const SEMANTIC_SCHEMA: u32 = 5;

pub(super) fn collect(
    input: &AnalysisInput,
    config: Option<&Table>,
    collect_hir: bool,
    collect_private_dead_code: bool,
    target_dir: &Path,
    facts: &mut CodebaseFacts,
    stop_on_private: Option<&mut dyn FnMut(&CodebaseFacts) -> bool>,
) -> Result<bool, AnalysisError> {
    let (config_file, mut command) = analyzer_command(
        input,
        config,
        collect_hir,
        collect_private_dead_code,
        stop_on_private.is_some(),
        target_dir,
    )?;
    let report = if let Some(stop) = stop_on_private {
        match stream::run(input, facts, &mut command, stop)? {
            stream::Outcome::Complete(report) => report,
            stream::Outcome::Stopped => return Ok(true),
        }
    } else {
        let output = command.output().map_err(|error| {
            AnalysisError::new(format!("could not run semantic analysis: {error}"))
        })?;
        if !output.status.success() {
            return Err(command_failure("semantic analysis", &output));
        }
        serde_json::from_slice(&output.stdout).map_err(|error| {
            AnalysisError::new(format!("semantic analyzer returned invalid JSON: {error}"))
        })?
    };
    drop(config_file);
    validate_report(&report)?;
    for diagnostic in report.diagnostics {
        append_diagnostic(input, facts, diagnostic)?;
    }
    Ok(false)
}

fn analyzer_command(
    input: &AnalysisInput,
    config: Option<&Table>,
    collect_hir: bool,
    collect_private_dead_code: bool,
    stream_private_dead_code: bool,
    target_dir: &Path,
) -> Result<(Option<tempfile::NamedTempFile>, Command), AnalysisError> {
    let (config_file, excluded_crates) = semantic_config_file(config)?;
    let executable = env::current_exe()
        .map_err(|error| AnalysisError::new(format!("could not locate Axiom: {error}")))?;
    let mut command = Command::new(executable);
    command
        .arg("__semantic")
        .arg("check")
        .arg("--manifest-path")
        .arg(input.workspace_root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target_dir)
        .args(["--output-format", "json", "--color", "never"])
        .args(["-W", "hawk::test_only"])
        .args(["-W", "hawk::unnecessary_crate_visibility"])
        .env("AXIOM_INTERNAL_SEMANTIC_CONFIG", "1")
        .env("RUSTUP_TOOLCHAIN", SEMANTIC_TOOLCHAIN);
    if collect_private_dead_code {
        command.env(
            policy_semantic::protocol::PRIVATE_DEAD_CODE_ENV,
            if collect_hir { "1" } else { "only" },
        );
    }
    if stream_private_dead_code {
        command.env(policy_semantic::protocol::PRIVATE_DEAD_CODE_STREAM_ENV, "1");
    }
    if let Some(file) = &config_file {
        command.arg("--config").arg(file.path());
    }
    if super::offline() {
        command.env("CARGO_NET_OFFLINE", "true");
    }
    for crate_name in excluded_crates {
        command.arg("--exclude-crate").arg(crate_name);
    }
    Ok((config_file, command))
}

fn validate_report(report: &SemanticReport) -> Result<(), AnalysisError> {
    if report.schema_version == SEMANTIC_SCHEMA {
        Ok(())
    } else {
        Err(AnalysisError::new(format!(
            "semantic analyzer schema {} is incompatible with expected schema {SEMANTIC_SCHEMA}",
            report.schema_version
        )))
    }
}

pub(super) fn append_diagnostic(
    input: &AnalysisInput,
    facts: &mut CodebaseFacts,
    diagnostic: RawSemanticDiagnostic,
) -> Result<(), AnalysisError> {
    if diagnostic.category == "configuration" {
        return Err(AnalysisError::new(format!(
            "semantic configuration {}: {}",
            diagnostic.code,
            diagnostic
                .reason
                .unwrap_or_else(|| "invalid or stale exception".to_owned())
        )));
    }
    let Some(kind) = hir_kind(&diagnostic.code) else {
        return Ok(());
    };
    let (Some(identity), Some(location)) = (diagnostic.identity, diagnostic.location) else {
        return Ok(());
    };
    let Some((path, span)) = source_location(input, &location) else {
        return Ok(());
    };
    push_unique(
        facts,
        SemanticFindingFact {
            kind,
            item: identity.item,
            item_kind: Some(identity.kind),
            path,
            span,
        },
    );
    Ok(())
}

pub(super) fn hir_kind(code: &str) -> Option<SemanticFindingKind> {
    match code {
        "hawk::dead_public" => Some(SemanticFindingKind::DeadPublic),
        "hawk::test_only" => Some(SemanticFindingKind::TestOnly),
        "hawk::unnecessary_public" => Some(SemanticFindingKind::UnnecessaryPublic),
        "hawk::unnecessary_restricted_visibility" => {
            Some(SemanticFindingKind::UnnecessaryRestrictedVisibility)
        }
        "hawk::unnecessary_crate_visibility" => {
            Some(SemanticFindingKind::UnnecessaryCrateVisibility)
        }
        "hawk::private_dead" => Some(SemanticFindingKind::PrivateDeadCode),
        _ => None,
    }
}
