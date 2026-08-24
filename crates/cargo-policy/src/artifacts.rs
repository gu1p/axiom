use core::hash::{Hash as _, Hasher as _};
use std::{
    collections::hash_map::DefaultHasher,
    env,
    path::{Path, PathBuf},
    process::Command,
};

const CARGO_TARGET_LAYOUT: &str = "cargo-target-v1";

/// Return Axiom's reusable Cargo target directory for one workspace.
///
/// An explicit target directory keeps build scripts, incremental state, and
/// compiler output out of both the inspected workspace and any user-configured
/// Cargo target directory. `env::temp_dir` honors `TMPDIR` on Unix platforms.
pub(crate) fn cargo_target_dir(workspace_root: &Path) -> PathBuf {
    cargo_target_dir_in(&env::temp_dir(), workspace_root)
}

pub(crate) fn configure_cargo(command: &mut Command, workspace_root: &Path) {
    command
        .arg("--target-dir")
        .arg(cargo_target_dir(workspace_root));
}

fn cargo_target_dir_in(temp_root: &Path, workspace_root: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    workspace_root.hash(&mut hasher);
    temp_root
        .join("axiom")
        .join(CARGO_TARGET_LAYOUT)
        .join(format!("{:016x}", hasher.finish()))
}

#[cfg(test)]
#[path = "tests/artifacts.rs"]
mod tests;
