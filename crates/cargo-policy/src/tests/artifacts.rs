use std::{ffi::OsStr, path::Path, process::Command};

use super::{CARGO_TARGET_LAYOUT, cargo_target_dir, cargo_target_dir_in, configure_cargo};

#[test]
fn cargo_artifacts_are_workspace_namespaced_below_platform_temp() {
    let workspace = Path::new("/workspaces/orders");
    let target = cargo_target_dir(workspace);

    assert!(target.starts_with(std::env::temp_dir()));
    assert_eq!(
        target.parent().and_then(Path::file_name),
        Some(CARGO_TARGET_LAYOUT.as_ref())
    );
    assert_eq!(target, cargo_target_dir(workspace));
    assert_ne!(
        target,
        cargo_target_dir(Path::new("/other/workspaces/orders"))
    );
}

#[test]
fn cargo_artifacts_honor_the_selected_temporary_root() {
    let temporary_root = Path::new("/selected-tmpdir");
    let target = cargo_target_dir_in(temporary_root, Path::new("/workspace"));

    assert!(target.starts_with(temporary_root.join("axiom")));
}

#[test]
fn cargo_artifacts_do_not_use_external_compiler_cache_wrappers() {
    let mut command = Command::new("cargo");
    configure_cargo(&mut command, Path::new("/workspace"));

    for variable in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
        assert!(
            command
                .get_envs()
                .any(|(key, value)| { key == variable && value == Some(OsStr::new("")) })
        );
    }
}
