mod support;

use std::process::Command;

use support::TestWorkspace;

fn assert_policy_contains(config: &str, expected: &str) {
    assert!(
        config.contains(expected),
        "generated policy should contain {expected:?}"
    );
}

fn assert_curated_clippy_profile(config: &str) {
    let policy: toml::Value = toml::from_str(config).expect("valid policy");
    let clippy = policy["tools"]["clippy"]
        .as_table()
        .expect("Clippy configuration");
    assert_eq!(
        clippy["profile"].as_str(),
        Some("axiom"),
        "generated policy should select the curated Axiom profile"
    );
    assert!(
        !clippy.contains_key("lints"),
        "generated policy should not contain a per-lint catalog"
    );
}

fn init_command(workspace: &TestWorkspace) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_axiom"));
    command
        .arg("init")
        .arg("--manifest-path")
        .arg(workspace.manifest());
    command
}

fn initialized_library_workspace() -> (TestWorkspace, String) {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 99, 999);
    std::fs::remove_file(workspace.root().join("policy.toml")).expect("remove fixture policy");

    let first = init_command(&workspace)
        .output()
        .expect("initialize policy");
    assert!(
        first.status.success(),
        "initialization should succeed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let config =
        std::fs::read_to_string(workspace.root().join("policy.toml")).expect("initialized policy");

    (workspace, config)
}

fn assert_default_size_rules(config: &str) {
    assert_policy_contains(config, "limit = 50");
    assert_policy_contains(config, "limit = 200");
    assert_policy_contains(
        config,
        "[rules.\"size/directory-max-files\"]\nlevel = \"deny\"\nlimit = 5\nscope = \"production\"",
    );
    assert_policy_contains(
        config,
        "[rules.\"size/directory-max-lines\"]\nlevel = \"deny\"\nlimit = 1000\nscope = \"production\"",
    );
}

fn assert_default_testing_rules(config: &str) {
    assert_policy_contains(config, "testing/separate-test-files");
    assert_policy_contains(config, "scope = \"production\"");
    assert_policy_contains(config, "test = [");
}

fn assert_default_clippy_tool(config: &str) {
    assert_policy_contains(config, "[tools.clippy]");
    assert_policy_contains(config, "profile = \"axiom\"");
    assert_policy_contains(config, "check-docs = true");
    assert_policy_contains(config, "warnings = \"deny\"");
    assert_curated_clippy_profile(config);
}

fn assert_repository_clippy_profile() {
    let repository_policy = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../policy.toml"),
    )
    .expect("repository policy");
    assert_curated_clippy_profile(&repository_policy);
    let clippy_config: toml::Value = toml::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../clippy.toml"),
        )
        .expect("repository Clippy configuration"),
    )
    .expect("valid repository Clippy configuration");
    assert_eq!(
        clippy_config["cognitive-complexity-threshold"].as_integer(),
        Some(12),
        "repository Clippy configuration should cap cognitive complexity at 12"
    );
}

fn assert_default_semantic_rules(config: &str) {
    assert_policy_contains(config, "[[semantic.production]]");
    assert_policy_contains(config, "# [rules.\"dead-code/public\"]");
    assert_policy_contains(
        config,
        "# [rules.\"visibility/unnecessary-public\"]\n# level = \"deny\"",
    );
}

fn assert_existing_policy_is_preserved(workspace: &TestWorkspace, expected: &str) {
    let second = init_command(workspace)
        .output()
        .expect("repeat initialization");
    assert_eq!(
        second.status.code(),
        Some(2),
        "repeated initialization should fail without overwriting"
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("could not create"),
        "repeated initialization should explain the existing policy: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(workspace.root().join("policy.toml")).expect("preserved policy"),
        expected,
        "repeated initialization should preserve the existing policy"
    );
}

#[test]
fn init_creates_the_default_policy_and_refuses_to_overwrite_it() {
    let (workspace, config) = initialized_library_workspace();

    assert_default_size_rules(&config);
    assert_default_testing_rules(&config);
    assert_default_clippy_tool(&config);
    assert_repository_clippy_profile();
    assert_default_semantic_rules(&config);
    assert_existing_policy_is_preserved(&workspace, &config);
}

#[test]
fn init_enables_semantic_rules_for_binary_workspaces() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 99, 999);
    std::fs::write(workspace.root().join("src/main.rs"), "fn main() {}\n").expect("binary target");
    std::fs::remove_file(workspace.root().join("policy.toml")).expect("remove fixture policy");

    let output = init_command(&workspace)
        .output()
        .expect("initialize binary policy");
    assert!(output.status.success());
    let config =
        std::fs::read_to_string(workspace.root().join("policy.toml")).expect("initialized policy");
    assert!(config.contains("[rules.\"dead-code/private\"]\nlevel = \"warn\""));
    assert!(config.contains("[rules.\"visibility/unnecessary-public\"]\nlevel = \"deny\""));
    assert!(config.contains("[rules.\"visibility/unnecessary-crate\"]"));
    assert!(!config.contains("[[semantic.production]]"));
}
