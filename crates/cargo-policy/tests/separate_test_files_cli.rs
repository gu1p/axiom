mod support;

use std::{fs, process::Command};

use serde_json::Value;
use support::TestWorkspace;

fn command(workspace: &TestWorkspace) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_axiom"));
    command
        .arg("check")
        .arg("--manifest-path")
        .arg(workspace.manifest())
        .arg("--color")
        .arg("never");
    command
}

fn enable_rule(workspace: &TestWorkspace) {
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[rules."testing/separate-test-files"]
level = "deny"
"#,
    )
    .expect("separate-test-files policy");
}

#[test]
fn inline_tests_in_production_files_are_rejected() {
    let workspace = TestWorkspace::new(
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n",
        "deny",
        50,
        200,
    );
    enable_rule(&workspace);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[testing/separate-test-files]"));
    assert!(stderr.contains("test module `tests` is implemented in a production file"));
    assert!(!stderr.contains("physical lines"));
}

#[test]
fn dedicated_test_files_and_external_test_modules_are_allowed() {
    let workspace = TestWorkspace::new(
        "#[cfg(test)]\n#[path = \"lib_tests.rs\"]\nmod tests;\npub fn value() -> u8 { 1 }\n",
        "deny",
        50,
        200,
    );
    fs::write(
        workspace.root().join("src/lib_tests.rs"),
        "use super::value;\n#[test]\nfn returns_value() { assert_eq!(value(), 1); }\n",
    )
    .expect("dedicated test file");
    enable_rule(&workspace);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("policy check passed (2 Rust files)"));
}

#[test]
fn invariant_json_diagnostics_do_not_report_metric_values() {
    let workspace = TestWorkspace::new("#[test]\nfn misplaced() {}\n", "deny", 50, 200);
    enable_rule(&workspace);
    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run cargo-policy");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(diagnostic["class"], "invariant");
    assert!(diagnostic["observed"].is_null());
    assert!(diagnostic["limit"].is_null());
}
