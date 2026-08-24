use policy_core::{AnalysisInput, ClippyConfig};

use super::command;

#[test]
fn command_keeps_rustdoc_artifacts_below_platform_temp() {
    let input = AnalysisInput {
        workspace_root: "/workspace".into(),
        sources: Vec::new(),
    };
    let command = command(&input, &ClippyConfig::default());
    let arguments: Vec<_> = command.get_args().collect();
    let target = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "--target-dir")
        .map(|arguments| std::path::Path::new(arguments[1]))
        .expect("rustdoc command has an explicit target directory");

    assert!(target.starts_with(std::env::temp_dir()));
}
