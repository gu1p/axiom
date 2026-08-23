use policy_core::{
    ClippyConfig, ClippyFeatureSelection, ClippyTargetCoverage, ClippyWarningPolicy, Level,
};

use super::{command, profile};

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
    assert!(!arguments.windows(2).any(|pair| pair == ["-D", "warnings"]));
    assert!(
        arguments
            .windows(2)
            .any(|pair| pair == ["-D", "unsafe-code"])
    );
    assert!(arguments.contains(&"clippy::pedantic".to_owned()));
    assert!(arguments.contains(&"clippy::unwrap-used".to_owned()));
    assert!(arguments.contains(&"clippy::cast-lossless".to_owned()));
}

#[test]
fn individual_lint_overrides_are_applied_last_to_the_correct_backend() {
    let mut config = ClippyConfig::default();
    config
        .lints
        .insert("clippy::unwrap_used".to_owned(), Level::Allow);
    config
        .lints
        .insert("rustdoc::broken_intra_doc_links".to_owned(), Level::Warn);

    let compiler = profile::compiler_arguments(&config);
    assert!(compiler.ends_with(&["-A".to_owned(), "clippy::unwrap_used".to_owned()]));
    assert!(!compiler.contains(&"rustdoc::broken_intra_doc_links".to_owned()));

    let rustdoc = profile::rustdoc_arguments(&config);
    assert!(rustdoc.ends_with(&[
        "--force-warn".to_owned(),
        "rustdoc::broken_intra_doc_links".to_owned(),
    ]));
    assert!(!rustdoc.contains(&"clippy::unwrap_used".to_owned()));
}
