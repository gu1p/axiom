use policy_core::{
    ClippyConfig, ClippyFeatureSelection, ClippyTargetCoverage, ClippyWarningPolicy, Level,
};

use super::{command, profile};

fn configured_command_arguments() -> Vec<String> {
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
    let command = command(
        &input,
        &config,
        false,
        std::path::Path::new("/temporary/cargo-target"),
    );
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn contains_pair(arguments: &[String], flag: &str, value: &str) -> bool {
    arguments.windows(2).any(|pair| pair == [flag, value])
}

fn assert_command_shape(arguments: &[String]) {
    assert!(arguments.starts_with(&[
        "clippy".to_owned(),
        "--manifest-path".to_owned(),
        "/workspace/Cargo.toml".to_owned(),
    ]));
    assert!(arguments.contains(&"--workspace".to_owned()));
    let target = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "--target-dir")
        .map(|arguments| std::path::Path::new(&arguments[1]))
        .expect("Clippy command has an explicit target directory");
    assert_eq!(target, std::path::Path::new("/temporary/cargo-target"));
    assert!(arguments.contains(&"--no-deps".to_owned()));
    assert!(arguments.contains(&"--keep-going".to_owned()));
    assert!(!arguments.contains(&"--all-targets".to_owned()));
}

fn assert_feature_selection(arguments: &[String]) {
    assert!(!arguments.contains(&"--all-features".to_owned()));
    assert!(contains_pair(arguments, "--features", "server,postgres"));
    assert!(arguments.contains(&"--no-default-features".to_owned()));
}

fn assert_lint_profile(arguments: &[String]) {
    assert!(!contains_pair(arguments, "-D", "warnings"));
    assert!(contains_pair(arguments, "-D", "unsafe-code"));
    for lint in [
        "clippy::all",
        "clippy::cargo",
        "clippy::pedantic",
        "clippy::cognitive-complexity",
    ] {
        assert!(contains_pair(arguments, "-W", lint));
    }
    assert!(arguments.contains(&"clippy::unwrap-used".to_owned()));
    assert!(contains_pair(arguments, "-A", "clippy::cast-lossless"));
}

#[test]
fn command_maps_clippy_configuration_to_cargo_arguments() {
    let arguments = configured_command_arguments();
    assert_command_shape(&arguments);
    assert_feature_selection(&arguments);
    assert_lint_profile(&arguments);
}

#[test]
fn fail_fast_command_is_single_job_without_keep_going() {
    let input = policy_core::AnalysisInput {
        workspace_root: "/workspace".into(),
        sources: Vec::new(),
    };
    let command = command(
        &input,
        &ClippyConfig::default(),
        true,
        std::path::Path::new("/temporary/cargo-target"),
    );
    let arguments: Vec<_> = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();

    assert!(!arguments.contains(&"--keep-going".to_owned()));
    assert!(arguments.windows(2).any(|pair| pair == ["--jobs", "1"]));
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
