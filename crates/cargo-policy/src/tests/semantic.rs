use std::sync::Arc;

use camino::Utf8PathBuf;
use policy_core::{AnalysisInput, LineIndex, RustEdition, SemanticFindingKind, SourceUnit};
use toml::{Table, Value};

use super::{
    config::{axiom_rule_to_hawk, normalize_overrides},
    hir::hir_kind,
    types::{RawLocation, source_location},
};

#[test]
fn maps_every_hir_rule_to_the_embedded_analyzer() {
    for (rule, code, kind) in [
        (
            "dead-code/public",
            "hawk::dead_public",
            SemanticFindingKind::DeadPublic,
        ),
        (
            "dead-code/test-only",
            "hawk::test_only",
            SemanticFindingKind::TestOnly,
        ),
        (
            "visibility/unnecessary-public",
            "hawk::unnecessary_public",
            SemanticFindingKind::UnnecessaryPublic,
        ),
        (
            "visibility/unnecessary-restricted",
            "hawk::unnecessary_restricted_visibility",
            SemanticFindingKind::UnnecessaryRestrictedVisibility,
        ),
        (
            "visibility/unnecessary-crate",
            "hawk::unnecessary_crate_visibility",
            SemanticFindingKind::UnnecessaryCrateVisibility,
        ),
    ] {
        assert_eq!(axiom_rule_to_hawk(rule), Some(code));
        assert_eq!(hir_kind(code), Some(kind));
    }
}

#[test]
fn rewrites_axiom_override_ids_for_the_internal_config() {
    let mut entry = Table::new();
    entry.insert(
        "rule".to_owned(),
        Value::String("dead-code/public".to_owned()),
    );
    let mut config = Table::new();
    config.insert(
        "override".to_owned(),
        Value::Array(vec![Value::Table(entry)]),
    );

    normalize_overrides(&mut config).expect("valid override");
    let entry = config["override"].as_array().expect("array")[0]
        .as_table()
        .expect("table");
    assert_eq!(entry["lint"].as_str(), Some("hawk::dead_public"));
    assert!(!entry.contains_key("rule"));
}

#[test]
fn accepts_only_locations_from_loaded_workspace_sources() {
    let text = "pub fn example() {}\n".to_owned();
    let input = AnalysisInput {
        workspace_root: Utf8PathBuf::from("/workspace"),
        sources: vec![SourceUnit {
            absolute_path: Utf8PathBuf::from("/workspace/src/lib.rs"),
            relative_path: Utf8PathBuf::from("src/lib.rs"),
            edition: RustEdition::Edition2024,
            lines: Arc::new(LineIndex::new(&text)),
            text: Arc::from(text),
        }],
    };
    let location = RawLocation {
        file: "src/lib.rs".to_owned(),
        byte_start: Some(4),
        byte_end: Some(6),
        line: Some(1),
        column: Some(5),
        end_line: Some(1),
        end_column: Some(7),
    };
    let (path, span) = source_location(&input, &location).expect("workspace location");
    assert_eq!(path, Utf8PathBuf::from("src/lib.rs"));
    assert_eq!(span.byte_start, 4);

    let external = RawLocation {
        file: "../dependency/src/lib.rs".to_owned(),
        ..location
    };
    assert!(source_location(&input, &external).is_none());
}
