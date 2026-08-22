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
