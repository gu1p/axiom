use policy_core::{
    ClippyConfig, ClippyFeatureSelection, ClippyTargetCoverage, ClippyWarningPolicy,
};

use super::command;

#[test]
fn command_maps_clippy_configuration_to_cargo_arguments() {
    let input = policy_core::AnalysisInput {
        workspace_root: "/workspace".into(),
        sources: Vec::new(),
    };
    let config = ClippyConfig {
        targets: ClippyTargetCoverage::Default,
        features: ClippyFeatureSelection::Selected(vec!["server".into(), "postgres".into()]),
        no_default_features: true,
        warnings: ClippyWarningPolicy::Warn,
        ..ClippyConfig::default()
    };
    let command = command(&input, &config);
    let arguments: Vec<_> = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();

    assert!(arguments.starts_with(&[
        "clippy".to_owned(),
        "--manifest-path".to_owned(),
        "/workspace/Cargo.toml".to_owned(),
    ]));
    assert!(arguments.contains(&"--workspace".to_owned()));
    assert!(arguments.contains(&"--no-deps".to_owned()));
    assert!(arguments.contains(&"--keep-going".to_owned()));
    assert!(!arguments.contains(&"--all-targets".to_owned()));
    assert!(!arguments.contains(&"--all-features".to_owned()));
    assert!(
        arguments
            .windows(2)
            .any(|pair| pair == ["--features", "server,postgres"])
    );
    assert!(arguments.contains(&"--no-default-features".to_owned()));
    assert!(!arguments.contains(&"-D".to_owned()));
}
