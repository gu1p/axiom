mod support;

use std::process::Command;

use support::TestWorkspace;

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
