mod config;
mod hir;
mod private_dead;
mod types;

use std::{env, process::Command};

use policy_core::{AnalysisError, AnalysisInput, CodebaseFacts, FactProvider};
use toml::Table;

pub(super) const SEMANTIC_TOOLCHAIN: &str = "1.98.0";

pub struct SemanticFactProvider {
    config: Option<Table>,
    collect_hir: bool,
    collect_private_dead_code: bool,
}

impl SemanticFactProvider {
    pub fn new(config: Option<Table>, collect_hir: bool, collect_private_dead_code: bool) -> Self {
        Self {
            config,
            collect_hir,
            collect_private_dead_code,
        }
    }
}

impl FactProvider for SemanticFactProvider {
    fn collect(
        &self,
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<(), Vec<AnalysisError>> {
        #[cfg(target_os = "linux")]
        ensure_supported_host().map_err(single_error)?;
        ensure_toolchain().map_err(single_error)?;
        if self.collect_hir {
            hir::collect(input, self.config.as_ref(), facts).map_err(single_error)?;
        }
        if self.collect_private_dead_code {
            private_dead::collect(input, facts).map_err(single_error)?;
        }
        Ok(())
    }
}

fn single_error(error: AnalysisError) -> Vec<AnalysisError> {
    vec![error]
}

#[cfg(target_os = "linux")]
fn ensure_supported_host() -> Result<(), AnalysisError> {
    if std::path::Path::new("/etc/alpine-release").exists() {
        return Err(AnalysisError::new(
            "semantic policies require a glibc Linux host; the static Axiom frontend remains usable on musl when semantic rules are disabled",
        ));
    }
    Ok(())
}

fn ensure_toolchain() -> Result<(), AnalysisError> {
    let probe = Command::new("rustup")
        .args(["run", SEMANTIC_TOOLCHAIN, "rustc", "-vV"])
        .output();
    if probe.as_ref().is_ok_and(|output| output.status.success()) {
        return Ok(());
    }
    if offline() {
        return Err(AnalysisError::new(format!(
            "Rust {SEMANTIC_TOOLCHAIN} is required for semantic policies and AXIOM_OFFLINE is set; run `rustup toolchain install {SEMANTIC_TOOLCHAIN} --profile minimal`"
        )));
    }
    eprintln!("installing Rust {SEMANTIC_TOOLCHAIN} for Axiom semantic policies...");
    install_toolchain()
}

fn install_toolchain() -> Result<(), AnalysisError> {
    let status = Command::new("rustup")
        .args([
            "toolchain",
            "install",
            SEMANTIC_TOOLCHAIN,
            "--profile",
            "minimal",
        ])
        .status()
        .map_err(|error| {
            AnalysisError::new(format!(
                "could not run rustup to install Rust {SEMANTIC_TOOLCHAIN}: {error}; install it manually with `rustup toolchain install {SEMANTIC_TOOLCHAIN} --profile minimal`"
            ))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(AnalysisError::new(format!(
            "rustup could not install Rust {SEMANTIC_TOOLCHAIN}; run `rustup toolchain install {SEMANTIC_TOOLCHAIN} --profile minimal`"
        )))
    }
}

fn offline() -> bool {
    env::var("AXIOM_OFFLINE")
        .is_ok_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

#[cfg(test)]
#[path = "tests/semantic.rs"]
mod tests;
