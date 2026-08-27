use std::{ffi::OsStr, process::Command};

use super::{RunArtifacts, configure_cargo};

impl RunArtifacts {
    fn new_in(parent: &std::path::Path) -> Result<Self, policy_core::AnalysisError> {
        tempfile::Builder::new()
            .prefix(super::RUN_PREFIX)
            .tempdir_in(parent)
            .map(|directory| Self { directory })
            .map_err(|error| policy_core::AnalysisError::new(error.to_string()))
    }
}

#[test]
fn run_artifacts_are_unique_and_removed_explicitly() {
    let parent = tempfile::tempdir().expect("temporary root");
    let first = RunArtifacts::new_in(parent.path()).expect("first run artifacts");
    let second = RunArtifacts::new_in(parent.path()).expect("second run artifacts");
    let first_target = first.cargo_target_dir();
    let second_target = second.cargo_target_dir();
    let first_root = first_target.parent().expect("artifact root").to_owned();

    assert_ne!(first_target, second_target);
    assert!(first_root.starts_with(parent.path()));
    assert_eq!(
        first.semantic_target_dir().parent(),
        Some(first_root.as_path())
    );
    first.cleanup().expect("clean first run artifacts");
    assert!(!first_root.exists());
    second.cleanup().expect("clean second run artifacts");
}

#[test]
fn cargo_artifacts_preserve_external_compiler_cache_wrappers() {
    let mut command = Command::new("cargo");
    command
        .env("RUSTC_WRAPPER", "kache")
        .env("RUSTC_WORKSPACE_WRAPPER", "workspace-wrapper");
    configure_cargo(&mut command, std::path::Path::new("/temporary/target"));

    assert!(
        command
            .get_envs()
            .any(|(key, value)| { key == "RUSTC_WRAPPER" && value == Some(OsStr::new("kache")) })
    );
    assert!(command.get_envs().any(|(key, value)| {
        key == "RUSTC_WORKSPACE_WRAPPER" && value == Some(OsStr::new("workspace-wrapper"))
    }));
    assert!(
        command
            .get_args()
            .any(|argument| argument == "/temporary/target")
    );
}
