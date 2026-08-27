use std::{path::Path, process::Command};

use policy_core::AnalysisError;

const RUN_PREFIX: &str = "axiom-run-";

/// Owns every compiler artifact created by one `axiom check` invocation.
///
/// The directory is unique, shared by all checks in the invocation, and
/// removed explicitly before Axiom returns. `TempDir` also provides cleanup
/// during unwinding if execution exits an error path unexpectedly.
pub(super) struct RunArtifacts {
    directory: tempfile::TempDir,
}

impl RunArtifacts {
    pub(super) fn new() -> Result<Self, AnalysisError> {
        tempfile::Builder::new()
            .prefix(RUN_PREFIX)
            .tempdir()
            .map(|directory| Self { directory })
            .map_err(|error| {
                AnalysisError::new(format!(
                    "could not create Axiom's temporary artifact directory: {error}"
                ))
            })
    }

    pub(super) fn cargo_target_dir(&self) -> std::path::PathBuf {
        self.directory.path().join("cargo-target")
    }

    pub(super) fn semantic_target_dir(&self) -> std::path::PathBuf {
        self.directory.path().join("semantic-target")
    }

    pub(super) fn cleanup(self) -> Result<(), AnalysisError> {
        let path = self.directory.path().to_path_buf();
        self.directory.close().map_err(|error| {
            AnalysisError::new(format!(
                "could not remove Axiom's temporary artifact directory {}: {error}",
                path.display()
            ))
        })
    }
}

pub(super) fn configure_cargo(command: &mut Command, target_dir: &Path) {
    command.arg("--target-dir").arg(target_dir);
}

#[cfg(test)]
#[path = "tests/artifacts.rs"]
mod tests;
