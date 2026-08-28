mod support;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use support::TestWorkspace;

const LINT_VALUES_COMMENT: &str =
    "# Possible values: \"deny\" (error), \"warn\" (warning), \"allow\" (disabled).";

fn clippy_lints(config: &str) -> BTreeMap<String, String> {
    let policy: toml::Value = toml::from_str(config).expect("valid generated policy");
    policy["tools"]["clippy"]["lints"]
        .as_table()
        .expect("generated Clippy lint catalog")
        .iter()
        .map(|(name, level)| {
            (
                name.clone(),
                level.as_str().expect("Clippy lint level").to_owned(),
            )
        })
        .collect()
}

fn assert_documented_clippy_catalog(config: &str) {
    let lints = clippy_lints(config);
    let mut previous = "";
    let mut documented = 0;
    for line in config.lines() {
        if line.starts_with("\"clippy::") {
            assert_eq!(
                previous, LINT_VALUES_COMMENT,
                "missing comment above {line}"
            );
            documented += 1;
        }
        previous = line;
    }
    assert_eq!(
        documented,
        lints.len(),
        "every catalog entry has a values comment"
    );
    assert_eq!(lints.len(), 822, "catalog covers the pinned Clippy");
    assert_eq!(
        lints["clippy::cognitive_complexity"], "deny",
        "cognitive complexity is enabled"
    );
    assert!(
        lints
            .values()
            .all(|level| level == "deny" || level == "allow"),
        "every lint is explicitly enabled or disabled"
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

#[test]
fn init_creates_the_default_policy_and_refuses_to_overwrite_it() {
    let workspace = TestWorkspace::new("pub fn small() {}\n", "deny", 99, 999);
    std::fs::remove_file(workspace.root().join("policy.toml")).expect("remove fixture policy");

    let first = init_command(&workspace)
        .output()
        .expect("initialize policy");
    assert!(first.status.success());
    let config =
        std::fs::read_to_string(workspace.root().join("policy.toml")).expect("initialized policy");
    assert!(config.contains("limit = 50"));
    assert!(config.contains("limit = 200"));
    assert!(config.contains(
        "[rules.\"size/directory-max-files\"]\nlevel = \"deny\"\nlimit = 5\nscope = \"production\""
    ));
    assert!(config.contains(
        "[rules.\"size/directory-max-lines\"]\nlevel = \"deny\"\nlimit = 1000\nscope = \"production\""
    ));
    assert!(config.contains("testing/separate-test-files"));
    assert!(config.contains("scope = \"production\""));
    assert!(config.contains("test = ["));
    assert!(config.contains("[tools.clippy]"));
    assert!(config.contains("profile = \"axiom\""));
    assert!(config.contains("check-docs = true"));
    assert!(config.contains("warnings = \"deny\""));
    assert_documented_clippy_catalog(&config);
    let repository_policy =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../policy.toml"))
            .expect("repository policy");
    assert_documented_clippy_catalog(&repository_policy);
    assert_eq!(clippy_lints(&config), clippy_lints(&repository_policy));
    assert!(config.contains("[[semantic.production]]"));
    assert!(config.contains("# [rules.\"dead-code/public\"]"));
    assert!(config.contains("# [rules.\"visibility/unnecessary-public\"]\n# level = \"deny\""));

    let second = init_command(&workspace)
        .output()
        .expect("repeat initialization");
    assert_eq!(second.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&second.stderr).contains("could not create"));
    assert_eq!(
        std::fs::read_to_string(workspace.root().join("policy.toml")).expect("preserved policy"),
        config
    );
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
