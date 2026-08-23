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
        .arg("never")
        .env("CARGO_TARGET_DIR", workspace.root().join("target"));
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
    assert!(
        lines >= 2,
        "a function fixture needs opening and closing lines"
    );
    let mut source = String::from("fn oversized() {\n");
    for _ in 0..lines - 2 {
        source.push_str("    work();\n");
    }
    source.push_str("}\nfn work() {}\n");
    source
}

fn write_scoped_function_policy(workspace: &TestWorkspace, scope: &str) {
    fs::write(
        workspace.root().join("policy.toml"),
        format!(
            r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = false

[rules."size/function-max-lines"]
level = "deny"
limit = 50
scope = "{scope}"
"#,
        ),
    )
    .expect("scoped policy");
}

#[test]
fn clean_workspace_passes_with_human_summary() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 50, 200);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("axiom check passed (1 Rust files)"));
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
    assert_eq!(
        document["diagnostics"][0]["configuration"]["key"],
        "rules.\"size/function-max-lines\".level"
    );
    assert_eq!(document["diagnostics"][0]["configuration"]["value"], "deny");
    assert_eq!(
        document["diagnostics"][0]["configuration"]["levels"]["allow"],
        "disabled"
    );
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
fn production_scope_ignores_dedicated_and_attributed_tests() {
    let attributed_test = format!("#[test]\n{}", oversized_function(51));
    let workspace = TestWorkspace::new(&attributed_test, "deny", 50, 200);
    fs::create_dir(workspace.root().join("tests")).expect("tests directory");
    fs::write(
        workspace.root().join("tests/large_tests.rs"),
        oversized_function(51),
    )
    .expect("dedicated test source");
    write_scoped_function_policy(&workspace, "production");

    let output = command(&workspace).output().expect("run scoped policy");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("axiom check passed"));
}

#[test]
fn test_scope_reports_only_test_code() {
    let workspace = TestWorkspace::new(&oversized_function(51), "deny", 50, 200);
    fs::create_dir(workspace.root().join("tests")).expect("tests directory");
    fs::write(
        workspace.root().join("tests/large_tests.rs"),
        oversized_function(51),
    )
    .expect("dedicated test source");
    write_scoped_function_policy(&workspace, "test");

    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run scoped policy");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["path"], "tests/large_tests.rs");
}

#[test]
fn rejects_unknown_rule_scopes() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 50, 200);
    write_scoped_function_policy(&workspace, "examples");

    let output = command(&workspace).output().expect("run scoped policy");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid scope"));
}

#[test]
fn warning_does_not_fail_the_check() {
    let workspace = TestWorkspace::new(&oversized_function(51), "warn", 50, 200);
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning[size/function-max-lines]"));
    assert!(
        stderr
            .contains("policy: rules.\"size/function-max-lines\".level = \"warn\" in policy.toml")
    );
    assert!(
        stderr.contains("configure: \"deny\" = error, \"warn\" = warning, \"allow\" = disabled")
    );
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
        concat!(
            "version = 1\n",
            "[sources]\n",
            "include = [\"**/*.rs\"]\n",
            "exclude = [\"src/**\"]\n",
            "[tools.clippy]\n",
            "enabled = false\n",
        ),
    )
    .expect("excluded policy");
    let output = command(&workspace).output().expect("run cargo-policy");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("axiom check passed (0 Rust files"));
}

#[test]
fn private_dead_code_uses_rustc_and_respects_source_allows() {
    let workspace = TestWorkspace::new(
        "fn unused_private() {}\n#[allow(dead_code)]\nfn intentional_hook() {}\n",
        "deny",
        50,
        200,
    );
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = false

[rules."dead-code/private"]
level = "warn"
"#,
    )
    .expect("semantic policy");

    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run private dead-code policy");
    assert!(output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule_id"], "dead-code/private");
    assert!(
        diagnostics[0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("unused_private"))
    );
}

fn write_clippy_policy(workspace: &TestWorkspace, enabled: bool, deny_warnings: bool) {
    fs::write(
        workspace.root().join("policy.toml"),
        format!(
            r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = {enabled}
check-docs = false
targets = "all"
warnings = "{}"
"#,
            if deny_warnings { "deny" } else { "warn" },
        ),
    )
    .expect("Clippy policy");
}

fn write_clippy_lint_override(workspace: &TestWorkspace, level: &str) {
    fs::write(
        workspace.root().join("policy.toml"),
        format!(
            r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = true
check-docs = false
warnings = "deny"

[tools.clippy.lints]
"clippy::needless_return" = "{level}"
"#,
        ),
    )
    .expect("per-lint Clippy policy");
}

#[test]
fn clippy_denied_warning_is_a_versioned_tool_diagnostic() {
    let workspace = TestWorkspace::new("pub fn answer() -> u8 { return 42; }\n", "deny", 50, 200);
    write_clippy_policy(&workspace, true, true);

    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run Clippy through Axiom");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(document["outcome"], "violations");
    assert_eq!(document["diagnostics"][0]["kind"], "tool");
    assert_eq!(document["diagnostics"][0]["tool"], "clippy");
    assert_eq!(
        document["diagnostics"][0]["rule_id"],
        "clippy::needless_return"
    );
    assert_eq!(
        document["diagnostics"][0]["configuration"]["key"],
        "tools.clippy.lints.\"clippy::needless_return\""
    );

    let human = command(&workspace)
        .output()
        .expect("render Clippy guidance");
    assert_eq!(human.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&human.stderr).contains(
        "policy: tools.clippy.lints.\"clippy::needless_return\" = \"deny\" in policy.toml"
    ));
}

#[test]
fn clippy_warning_can_be_non_blocking_or_disabled() {
    let workspace = TestWorkspace::new("pub fn answer() -> u8 { return 42; }\n", "deny", 50, 200);
    write_clippy_policy(&workspace, true, false);

    let warning = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run non-blocking Clippy");
    assert!(
        warning.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&warning.stdout),
        String::from_utf8_lossy(&warning.stderr)
    );
    let document: Value = serde_json::from_slice(&warning.stdout).expect("valid JSON");
    assert_eq!(document["outcome"], "passed");
    assert_eq!(document["summary"]["warnings"], 1);

    write_clippy_policy(&workspace, false, true);
    let disabled = command(&workspace).output().expect("run without Clippy");
    assert!(disabled.status.success());
    assert!(String::from_utf8_lossy(&disabled.stderr).contains("axiom check passed"));
}

#[test]
fn individual_clippy_lint_can_be_demoted_or_disabled() {
    let workspace = TestWorkspace::new("pub fn answer() -> u8 { return 42; }\n", "deny", 50, 200);
    write_clippy_lint_override(&workspace, "warn");

    let warning = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run demoted Clippy lint");
    assert!(
        warning.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&warning.stdout),
        String::from_utf8_lossy(&warning.stderr)
    );
    let document: Value = serde_json::from_slice(&warning.stdout).expect("valid JSON");
    assert_eq!(document["summary"]["warnings"], 1);
    assert_eq!(document["diagnostics"][0]["configuration"]["value"], "warn");

    write_clippy_lint_override(&workspace, "allow");
    let allowed = command(&workspace)
        .output()
        .expect("run allowed Clippy lint");
    assert!(allowed.status.success());
    assert!(String::from_utf8_lossy(&allowed.stderr).contains("axiom check passed"));
}

#[test]
fn axiom_profile_enables_cherry_picked_clippy_lints() {
    let workspace = TestWorkspace::new(
        "//! Fixture docs.\npub fn debug_value() { dbg!(42); }\n",
        "deny",
        50,
        200,
    );
    write_clippy_policy(&workspace, true, true);

    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run Axiom Clippy profile");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|item| item["rule_id"] == "clippy::dbg_macro")
    );
}

#[test]
fn axiom_profile_checks_rustdoc_lints() {
    let workspace = TestWorkspace::new("pub fn answer() -> u8 { 42 }\n", "deny", 50, 200);
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
profile = "axiom"
check-docs = true
warnings = "deny"
"#,
    )
    .expect("rustdoc policy");

    let output = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("run rustdoc through Axiom");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|item| item["tool"] == "rustdoc"
                && item["rule_id"] == "rustdoc::missing_crate_level_docs")
    );
    let rustdoc = document["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .find(|item| item["tool"] == "rustdoc")
        .expect("rustdoc diagnostic");
    assert_eq!(
        rustdoc["configuration"]["key"],
        "tools.clippy.lints.\"rustdoc::missing_crate_level_docs\""
    );
}
