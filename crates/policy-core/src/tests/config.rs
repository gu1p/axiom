use std::fs;

use super::PolicyConfig;

fn load(text: &str) -> Result<PolicyConfig, super::ConfigError> {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = camino::Utf8PathBuf::from_path_buf(directory.path().join("policy.toml"))
        .expect("UTF-8 path");
    fs::write(&path, text).expect("configuration fixture");
    PolicyConfig::load(&path)
}

#[test]
fn validates_schema_version_and_includes() {
    let version = load("version = 2")
        .expect_err("unsupported version")
        .to_string();
    assert!(version.contains("version 2"));
    let includes = load("version = 1\n[sources]\ninclude = []")
        .expect_err("empty includes")
        .to_string();
    assert!(includes.contains("at least one glob"));
}

#[test]
fn rejects_unknown_top_level_fields() {
    let error = load("version = 1\nfuture = true")
        .expect_err("unknown key")
        .to_string();
    assert!(error.contains("unknown field"));
}

#[test]
fn preserves_native_semantic_configuration() {
    let config = load(
        r#"version = 1

[[semantic.production]]
package = "app"
bin = "app"
reason = "shipped binary"
"#,
    )
    .expect("semantic configuration");
    let semantic = config.semantic.expect("semantic table");
    assert!(semantic.contains_key("production"));
}

#[test]
fn clippy_is_enabled_with_strict_workspace_defaults() {
    let config = load("version = 1").expect("default configuration");

    assert!(config.tools.clippy.enabled);
    assert!(config.tools.clippy.uses_axiom_profile());
    assert!(config.tools.clippy.check_docs);
    assert!(config.tools.clippy.checks_all_targets());
    assert!(!config.tools.clippy.checks_all_features());
    assert!(!config.tools.clippy.no_default_features);
    assert!(config.tools.clippy.selected_features().is_none());
    assert!(config.tools.clippy.denies_warnings());
}

#[test]
fn parses_clippy_coverage_and_rejects_conflicting_features() {
    let config = load(
        r#"version = 1

[tools.clippy]
enabled = true
profile = "workspace"
check-docs = false
targets = "default"
features = ["server", "postgres"]
warnings = "warn"
"#,
    )
    .expect("custom Clippy configuration");
    assert!(!config.tools.clippy.uses_axiom_profile());
    assert!(!config.tools.clippy.check_docs);
    assert!(!config.tools.clippy.checks_all_targets());
    assert_eq!(
        config.tools.clippy.selected_features(),
        Some(["server".to_owned(), "postgres".to_owned()].as_slice())
    );
    assert!(!config.tools.clippy.denies_warnings());

    let error = load(
        r#"version = 1
[tools.clippy]
features = "all"
no-default-features = true
"#,
    )
    .expect_err("conflicting feature selection")
    .to_string();
    assert!(error.contains("cannot combine features = \"all\""));
}
