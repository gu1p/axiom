mod config;
mod hir;
mod types;

use std::io::Write as _;
use std::{env, fs, path::Path, process::Command};

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
        if self.collect_hir || self.collect_private_dead_code {
            hir::collect(
                input,
                self.config.as_ref(),
                self.collect_hir,
                self.collect_private_dead_code,
                facts,
            )
            .map_err(single_error)?;
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
    if toolchain_ready() {
        return Ok(());
    }
    if offline() {
        return Err(AnalysisError::new(format!(
            "Rust {SEMANTIC_TOOLCHAIN} with rustc-dev is required for semantic policies and AXIOM_OFFLINE is set; run `rustup toolchain install {SEMANTIC_TOOLCHAIN} --profile minimal --component rustc-dev`"
        )));
    }
    let _ = writeln!(
        std::io::stderr(),
        "installing Rust {SEMANTIC_TOOLCHAIN} with rustc-dev for Axiom semantic policies..."
    );
    install_toolchain()
}

fn toolchain_ready() -> bool {
    let output = Command::new("rustup")
        .args(["run", SEMANTIC_TOOLCHAIN, "rustc", "--print=sysroot"])
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(sysroot) = String::from_utf8(output.stdout) else {
        return false;
    };
    driver_library_available(Path::new(sysroot.trim()))
}

fn driver_library_available(sysroot: &Path) -> bool {
    #[cfg(windows)]
    let directory = sysroot.join("bin");
    #[cfg(not(windows))]
    let directory = sysroot.join("lib");
    fs::read_dir(directory).is_ok_and(|entries| {
        entries
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains("rustc_driver"))
    })
}

fn install_toolchain() -> Result<(), AnalysisError> {
    let status = Command::new("rustup")
        .args([
            "toolchain",
            "install",
            SEMANTIC_TOOLCHAIN,
            "--profile",
            "minimal",
            "--component",
            "rustc-dev",
        ])
        .status()
        .map_err(|error| {
            AnalysisError::new(format!(
                "could not run rustup to install Rust {SEMANTIC_TOOLCHAIN} with rustc-dev: {error}; install it manually with `rustup toolchain install {SEMANTIC_TOOLCHAIN} --profile minimal --component rustc-dev`"
            ))
        })?;
    if status.success() && toolchain_ready() {
        Ok(())
    } else {
        Err(AnalysisError::new(format!(
            "rustup could not install Rust {SEMANTIC_TOOLCHAIN} with rustc-dev; run `rustup toolchain install {SEMANTIC_TOOLCHAIN} --profile minimal --component rustc-dev`"
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
