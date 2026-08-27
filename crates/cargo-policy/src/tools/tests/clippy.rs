use std::collections::BTreeMap;
use std::process::Command;

use policy_core::{
    ClippyConfig, ClippyFeatureSelection, ClippyTargetCoverage, ClippyWarningPolicy, Level,
};

use super::{command, profile};

struct ClippyInventory {
    defaults: BTreeMap<String, Level>,
    groups: BTreeMap<String, Vec<String>>,
}

fn clippy_inventory() -> ClippyInventory {
    let output = Command::new("clippy-driver")
        .args(["-W", "help"])
        .output()
        .expect("run the pinned Clippy driver");
    assert!(output.status.success(), "clippy-driver -W help succeeds");
    let help = String::from_utf8(output.stdout).expect("Clippy help is UTF-8");
    let loaded = help
        .split_once("Lint checks loaded by this crate:")
        .expect("Clippy lint table")
        .1;
    let (checks, groups) = loaded
        .split_once("Lint groups loaded by this crate:")
        .expect("Clippy group table");
    ClippyInventory {
        defaults: parse_lint_levels(checks),
        groups: parse_lint_groups(groups),
    }
}

fn parse_lint_levels(checks: &str) -> BTreeMap<String, Level> {
    checks
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.strip_prefix("clippy::")?;
            let level = parse_level(fields.next().expect("Clippy lint default level"));
            Some((format!("clippy::{name}"), level))
        })
        .collect()
}

fn parse_lint_groups(groups: &str) -> BTreeMap<String, Vec<String>> {
    groups
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?.strip_prefix("clippy::")?;
            let members = fields
                .map(|member| member.trim_end_matches(',').to_owned())
                .collect();
            Some((format!("clippy::{name}"), members))
        })
        .collect()
}

fn parse_level(level: &str) -> Level {
    match level {
        "allow" => Level::Allow,
        "warn" => Level::Warn,
        "deny" => Level::Deny,
        other => panic!("unexpected Clippy lint level: {other}"),
    }
}

fn axiom_profile_levels(inventory: &ClippyInventory) -> BTreeMap<String, Level> {
    let mut levels = inventory.defaults.clone();
    let arguments = profile::compiler_arguments(&ClippyConfig::default());
    let (pairs, remainder) = arguments.as_chunks::<2>();
    assert!(
        remainder.is_empty(),
        "compiler arguments are level/lint pairs"
    );
    for pair in pairs {
        apply_compiler_argument(&mut levels, inventory, &pair[0], &pair[1]);
    }
    levels
        .into_iter()
        .map(|(name, level)| (name.replace('-', "_"), level))
        .collect()
}

fn apply_compiler_argument(
    levels: &mut BTreeMap<String, Level>,
    inventory: &ClippyInventory,
    flag: &str,
    lint: &str,
) {
    if lint == "warnings" && flag == "-D" {
        for level in levels.values_mut() {
            if *level == Level::Warn {
                *level = Level::Deny;
            }
        }
        return;
    }
    let Some(level) = level_from_flag(flag) else {
        return;
    };
    if let Some(members) = inventory.groups.get(lint) {
        for member in members {
            levels.insert(member.clone(), level);
        }
    } else if lint.starts_with("clippy::") {
        levels.insert(lint.to_owned(), level);
    }
}

fn level_from_flag(flag: &str) -> Option<Level> {
    match flag {
        "-A" => Some(Level::Allow),
        "-W" | "--force-warn" => Some(Level::Warn),
        "-D" => Some(Level::Deny),
        _ => None,
    }
}

fn catalog_levels() -> BTreeMap<String, Level> {
    let catalog: toml::Value = toml::from_str(include_str!("../../clippy-lints.toml"))
        .expect("built-in Clippy lint catalog is valid TOML");
    catalog["tools"]["clippy"]["lints"]
        .as_table()
        .expect("Clippy lint table")
        .iter()
        .map(|(name, value)| {
            let level = value.as_str().expect("Clippy lint level");
            (name.clone(), parse_level(level))
        })
        .collect()
}

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
    let command = command(
        &input,
        &config,
        false,
        std::path::Path::new("/temporary/cargo-target"),
    );
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
    let target = arguments
        .windows(2)
        .find(|arguments| arguments[0] == "--target-dir")
        .map(|arguments| std::path::Path::new(&arguments[1]))
        .expect("Clippy command has an explicit target directory");
    assert_eq!(target, std::path::Path::new("/temporary/cargo-target"));
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
    assert!(arguments.contains(&"clippy::cognitive-complexity".to_owned()));
    assert!(arguments.contains(&"clippy::unwrap-used".to_owned()));
    assert!(arguments.contains(&"clippy::cast-lossless".to_owned()));
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

#[test]
fn complete_lint_catalog_matches_the_pinned_clippy_and_axiom_profile() {
    let inventory = clippy_inventory();
    let catalog = catalog_levels();

    assert_eq!(catalog.len(), 822);
    assert_eq!(catalog, axiom_profile_levels(&inventory));
    assert_eq!(catalog["clippy::cognitive_complexity"], Level::Deny);
}
