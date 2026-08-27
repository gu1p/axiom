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
    #[cfg(unix)]
    command.env("TMPDIR", workspace.root());
    #[cfg(windows)]
    command.env("TEMP", workspace.root());
    command
}

fn assert_artifacts_are_cleaned(workspace: &TestWorkspace) {
    let residue = fs::read_dir(workspace.root())
        .expect("read fixture temporary directory")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name == "axiom" || name.starts_with("axiom-run-") || name.starts_with("axiom-semantic-")
        })
        .collect::<Vec<_>>();
    assert!(
        residue.is_empty(),
        "Axiom left owned temporary artifacts behind: {residue:?}"
    );
    assert!(
        !workspace.root().join("target").exists(),
        "Axiom must ignore an inherited Cargo target directory"
    );
}

#[test]
fn reports_the_compiled_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_axiom"))
        .arg("--version")
        .output()
        .expect("run axiom");
    assert!(
        output.status.success(),
        "axiom failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("axiom: checking policies..."));
    assert!(stderr.contains("axiom: finished policies in "));
    assert!(stderr.contains("axiom check passed (1 Rust files)"));
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
    assert_artifacts_are_cleaned(&workspace);
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
    assert!(
        output.status.success(),
        "axiom failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule_id"], "dead-code/private");
    assert!(
        diagnostics[0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("unused_private"))
    );
    assert_artifacts_are_cleaned(&workspace);

    let second = command(&workspace)
        .args(["--format", "json"])
        .output()
        .expect("rerun private dead-code policy");
    assert!(second.status.success());
    let second_document: Value = serde_json::from_slice(&second.stdout).expect("valid JSON");
    assert_eq!(
        second_document["diagnostics"].as_array().map(Vec::len),
        Some(1)
    );
    assert_artifacts_are_cleaned(&workspace);
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
    assert_artifacts_are_cleaned(&workspace);
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
fn axiom_profile_enforces_cognitive_complexity() {
    let conditions = (0..26)
        .map(|index| format!("    if bits[{index}] {{ count += 1; }}"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "//! Fixture docs.\npub fn count(bits: [bool; 26]) -> usize {{\n    let mut count = 0;\n{conditions}\n    count\n}}\n"
    );
    let workspace = TestWorkspace::new(&source, "deny", 50, 200);
    write_clippy_policy(&workspace, true, true);

    let output = command(&workspace)
        .args(["--clippy", "--format", "json"])
        .output()
        .expect("run Axiom cognitive complexity lint");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|item| item["rule_id"] == "clippy::cognitive_complexity")
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
    assert_artifacts_are_cleaned(&workspace);
}

fn write_size_and_testing_policy(workspace: &TestWorkspace) {
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = false

[rules."size/function-max-lines"]
level = "warn"
limit = 2

[rules."testing/separate-test-files"]
level = "deny"
"#,
    )
    .expect("size and testing policy");
}

#[test]
fn family_selectors_isolate_and_union_native_policies() {
    let workspace = TestWorkspace::new(
        "#[test]\nfn misplaced() {\n    work();\n}\nfn work() {}\n",
        "deny",
        50,
        200,
    );
    write_size_and_testing_policy(&workspace);

    let size = command(&workspace)
        .args(["--size", "--format", "json"])
        .output()
        .expect("run size family");
    assert!(size.status.success());
    let size: Value = serde_json::from_slice(&size.stdout).expect("size JSON");
    assert_eq!(size["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(size["diagnostics"][0]["rule_id"], "size/function-max-lines");

    let testing = command(&workspace)
        .args(["--testing", "--format", "json"])
        .output()
        .expect("run testing family");
    assert_eq!(testing.status.code(), Some(1));
    let testing: Value = serde_json::from_slice(&testing.stdout).expect("testing JSON");
    assert_eq!(testing["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        testing["diagnostics"][0]["rule_id"],
        "testing/separate-test-files"
    );

    let both = command(&workspace)
        .args(["--size", "--testing", "--format", "json"])
        .output()
        .expect("run selected families");
    let both: Value = serde_json::from_slice(&both.stdout).expect("union JSON");
    assert_eq!(both["diagnostics"].as_array().map(Vec::len), Some(2));
}

#[test]
fn ignore_warnings_suppresses_output_in_comprehensive_and_fail_fast_modes() {
    let workspace = TestWorkspace::new(
        "#[test]\nfn misplaced() {\n    work();\n}\nfn work() {}\n",
        "deny",
        50,
        200,
    );
    write_size_and_testing_policy(&workspace);

    let comprehensive = command(&workspace)
        .args(["--size", "--ignore-warnings", "--format", "json"])
        .output()
        .expect("ignore comprehensive warnings");
    assert!(comprehensive.status.success());
    let comprehensive: Value =
        serde_json::from_slice(&comprehensive.stdout).expect("comprehensive JSON");
    assert_eq!(comprehensive["summary"]["warnings"], 0);
    assert_eq!(
        comprehensive["diagnostics"].as_array().map(Vec::len),
        Some(0)
    );

    let fail_fast = command(&workspace)
        .args(["--fail-fast", "--ignore-warnings", "--format", "json"])
        .output()
        .expect("ignore fail-fast warnings");
    assert_eq!(fail_fast.status.code(), Some(1));
    let fail_fast: Value = serde_json::from_slice(&fail_fast.stdout).expect("fail-fast JSON");
    assert_eq!(fail_fast["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        fail_fast["diagnostics"][0]["rule_id"],
        "testing/separate-test-files"
    );
}

#[test]
fn fail_fast_warning_is_a_failure_and_prevents_heavier_checks() {
    let workspace = TestWorkspace::new(
        "#[test]\nfn misplaced() {\n    work();\n}\nfn work() {}\n",
        "deny",
        50,
        200,
    );
    write_size_and_testing_policy(&workspace);

    let output = command(&workspace)
        .args(["--fail-fast", "--format", "json"])
        .output()
        .expect("run fail-fast warning");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("fail-fast warning JSON");
    assert_eq!(document["outcome"], "violations");
    assert_eq!(document["summary"]["errors"], 0);
    assert_eq!(document["summary"]["warnings"], 1);
    assert_eq!(document["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        document["diagnostics"][0]["rule_id"],
        "size/function-max-lines"
    );
    assert!(!workspace.root().join("axiom/cargo-target-v1").exists());
}

#[test]
fn explicit_selector_does_not_enable_disabled_tools() {
    let workspace = TestWorkspace::new(&oversized_function(51), "deny", 50, 200);
    let output = command(&workspace)
        .arg("--clippy")
        .output()
        .expect("select disabled Clippy");
    assert!(output.status.success());
    assert!(!workspace.root().join("axiom/cargo-target-v1").exists());
}

#[test]
fn unselected_rule_configuration_is_still_validated() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 50, 200);
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[tools.clippy]
enabled = false

[rules."size/file-max-lines"]
level = "deny"
limit = 200

[rules."visibility/unnecessary-public"]
level = "deny"
unexpected = true
"#,
    )
    .expect("invalid unselected policy");

    let output = command(&workspace)
        .arg("--size")
        .output()
        .expect("validate unselected policy");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected"));
}

#[test]
fn unselected_semantic_configuration_is_still_validated() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 50, 200);
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[tools.clippy]
enabled = false

[semantic]
unexpected = true

[rules."size/file-max-lines"]
level = "deny"
limit = 200
"#,
    )
    .expect("invalid unselected semantic policy");

    let output = command(&workspace)
        .arg("--size")
        .output()
        .expect("validate unselected semantic policy");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected"));
}

#[test]
fn fail_fast_private_dead_code_cleans_before_the_next_run() {
    let workspace = TestWorkspace::new("fn unused_private() {}\n", "deny", 50, 200);
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = false

[rules."dead-code/private"]
level = "deny"
"#,
    )
    .expect("private dead-code policy");

    let first = command(&workspace)
        .args(["--dead-code", "--fail-fast", "--format", "json"])
        .output()
        .expect("fail fast on private dead code");
    assert_eq!(
        first.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let first: Value = serde_json::from_slice(&first.stdout).expect("fail-fast semantic JSON");
    assert_eq!(first["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(first["diagnostics"][0]["rule_id"], "dead-code/private");
    assert_artifacts_are_cleaned(&workspace);

    let full = command(&workspace)
        .args(["--dead-code", "--format", "json"])
        .output()
        .expect("rerun complete private dead-code analysis");
    assert_eq!(full.status.code(), Some(1));
    let full: Value = serde_json::from_slice(&full.stdout).expect("complete semantic JSON");
    assert_eq!(full["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_artifacts_are_cleaned(&workspace);
}

#[test]
fn fail_fast_clippy_streams_only_one_diagnostic() {
    let workspace = TestWorkspace::new(
        "//! Fixture docs.\n/// Return an answer.\npub fn answer() -> u8 { dbg!(42); return 42; }\n",
        "deny",
        50,
        200,
    );
    write_clippy_policy(&workspace, true, true);

    let output = command(&workspace)
        .args(["--clippy", "--fail-fast", "--format", "json"])
        .output()
        .expect("run fail-fast Clippy");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("fail-fast Clippy JSON");
    assert_eq!(document["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(document["diagnostics"][0]["tool"], "clippy");
}

#[test]
fn fail_fast_runs_clippy_before_collecting_semantic_syntax() {
    let workspace = TestWorkspace::new(
        "//! Fixture docs.\n/// Return an answer.\npub fn answer() -> u8 { return 42; }\n",
        "deny",
        50,
        200,
    );
    fs::write(workspace.root().join("src/ignored.rs"), "fn broken( {\n")
        .expect("uncompiled invalid source");
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[sources]
include = ["**/*.rs"]
exclude = []

[tools.clippy]
enabled = true
check-docs = false
targets = "all"
warnings = "deny"

[rules."dead-code/public"]
level = "deny"
"#,
    )
    .expect("Clippy and semantic policy");

    let output = command(&workspace)
        .args(["--fail-fast", "--clippy", "--dead-code", "--format", "json"])
        .output()
        .expect("run Clippy before semantic syntax");
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).expect("fail-fast Clippy JSON");
    assert_eq!(document["diagnostics"].as_array().map(Vec::len), Some(1));
    assert_eq!(document["diagnostics"][0]["tool"], "clippy");
}

#[test]
fn rustdoc_selector_does_not_run_clippy() {
    let workspace = TestWorkspace::new("pub fn answer() -> u8 { return 42; }\n", "deny", 50, 200);
    fs::write(
        workspace.root().join("policy.toml"),
        r#"version = 1

[tools.clippy]
profile = "axiom"
check-docs = true
warnings = "deny"
"#,
    )
    .expect("rustdoc-only policy");

    let output = command(&workspace)
        .args(["--rustdoc", "--format", "json"])
        .output()
        .expect("run only rustdoc");
    assert_eq!(output.status.code(), Some(1));
    let document: Value = serde_json::from_slice(&output.stdout).expect("rustdoc-only JSON");
    assert!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .all(|item| item["tool"] == "rustdoc")
    );
}
