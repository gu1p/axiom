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

#[test]
fn reports_the_compiled_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_axiom"))
        .arg("--version")
        .output()
        .expect("run axiom");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("axiom {}", env!("CARGO_PKG_VERSION"))
    );
}

fn oversized_function(lines: usize) -> String {
    assert!(lines >= 2);
    let mut source = String::from("fn oversized() {\n");
    for _ in 0..lines - 2 {
        source.push_str("    work();\n");
    }
    source.push_str("}\nfn work() {}\n");
    source
}

#[test]
fn clean_workspace_passes_with_human_summary() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 50, 200);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("policy check passed (1 Rust files)"));
}

#[test]
fn function_over_limit_is_a_versioned_json_violation() {
    let workspace = TestWorkspace::new(&oversized_function(51), "deny", 50, 200);
    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run cargo-policy");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["outcome"], "violations");
    assert_eq!(
        document["diagnostics"][0]["rule_id"],
        "size/function-max-lines"
    );
    assert_eq!(document["diagnostics"][0]["observed"], 51);
    assert_eq!(document["diagnostics"][0]["limit"], 50);
}

#[test]
fn function_limit_is_inclusive() {
    for lines in [49, 50] {
        let workspace = TestWorkspace::new(&oversized_function(lines), "deny", 50, 200);
        let output = command(&workspace).output().expect("run cargo-policy");
        assert!(output.status.success(), "{lines} lines should pass");
    }
}

#[test]
fn warning_does_not_fail_the_check() {
    let workspace = TestWorkspace::new(&oversized_function(51), "warn", 50, 200);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning[size/function-max-lines]"));
    assert!(stderr.contains("0 error(s) and 1 warning(s)"));
}

#[test]
fn file_over_limit_reports_the_file_rule() {
    let source = format!("{}pub fn small() {{}}\n", "// padding\n".repeat(200));
    let workspace = TestWorkspace::new(&source, "deny", 50, 200);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[size/file-max-lines]"));
}

#[test]
fn file_limit_is_inclusive() {
    for lines in [199, 200] {
        let source = format!("{}pub fn small() {{}}\n", "// padding\n".repeat(lines - 1));
        let workspace = TestWorkspace::new(&source, "deny", 50, 200);
        let output = command(&workspace).output().expect("run cargo-policy");
        assert!(output.status.success(), "{lines} lines should pass");
    }
}

#[test]
fn parse_failure_is_operational_and_suppresses_policy_results() {
    let workspace = TestWorkspace::new("fn broken( {\n", "deny", 1, 1);
    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run cargo-policy");
    assert_eq!(output.status.code(), Some(2));
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(document["outcome"], "error");
    assert_eq!(document["diagnostics"][0]["kind"], "operational");
    assert!(document["diagnostics"][0]["rule_id"].is_null());
}

#[test]
fn source_exclusions_are_workspace_relative() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 50, 200);
    fs::write(
        workspace.root().join("policy.toml"),
        "version = 1\n[sources]\ninclude = [\"**/*.rs\"]\nexclude = [\"src/**\"]\n",
    )
    .expect("excluded policy");
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("policy check passed (0 Rust files)"));
}
