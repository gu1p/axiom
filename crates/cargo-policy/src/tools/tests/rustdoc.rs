use policy_core::{AnalysisInput, ClippyConfig};

use super::command;

#[test]
fn command_uses_the_owned_run_target_directory() {
    let input = AnalysisInput {
        workspace_root: "/workspace".into(),
        sources: Vec::new(),
    };
    let command = command(
        &input,
        &ClippyConfig::default(),
        false,
        std::path::Path::new("/temporary/cargo-target"),
    );
    let arguments: Vec<_> = command.get_args().collect();
    let target = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "--target-dir")
        .map(|arguments| std::path::Path::new(arguments[1]))
        .expect("rustdoc command has an explicit target directory");

    assert_eq!(target, std::path::Path::new("/temporary/cargo-target"));
}
