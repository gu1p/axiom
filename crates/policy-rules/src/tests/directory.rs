use policy_core::{
    CodebaseFacts, Diagnostic, Level, MetricUnit, Rule as _, RuleScope, RustEdition,
    SourceFileFact, SourceKind, SourceUnit, TestCodeFact, TestCodeKind,
};

use super::{DirectoryLimit, DirectoryMetric};
use crate::size::limit::LimitConfig;

#[test]
fn groups_only_files_with_the_same_exact_parent_directory() {
    let facts = facts(vec![
        file("src/domain/nested/c.rs", SourceKind::Production, 1, false),
        file("src/domain/b.rs", SourceKind::Production, 1, false),
        file("src/domainish/d.rs", SourceKind::Production, 1, false),
        file("src/domain/a.rs", SourceKind::Production, 1, false),
    ]);

    let diagnostics = evaluate(DirectoryMetric::Files, 1, RuleScope::All, &facts);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].path.as_str(), "src/domain/b.rs");
    assert_eq!(diagnostics[0].observed, Some(2));
    assert_eq!(diagnostics[0].unit, Some(MetricUnit::Files));
    assert!(diagnostics[0].message.contains("`src/domain`"));
}

#[test]
fn scopes_use_whole_file_classification() {
    let facts = facts(vec![
        file("src/domain/a.rs", SourceKind::Production, 1, true),
        file("src/domain/b.rs", SourceKind::Production, 1, false),
        file("src/domain/c.rs", SourceKind::Test, 1, false),
        file("src/domain/d.rs", SourceKind::Test, 1, false),
    ]);

    let production = evaluate(DirectoryMetric::Files, 1, RuleScope::Production, &facts);
    let test = evaluate(DirectoryMetric::Files, 1, RuleScope::Test, &facts);
    let all = evaluate(DirectoryMetric::Files, 1, RuleScope::All, &facts);

    assert_eq!(production[0].observed, Some(2));
    assert_eq!(production[0].path.as_str(), "src/domain/b.rs");
    assert_eq!(test[0].observed, Some(2));
    assert_eq!(test[0].path.as_str(), "src/domain/d.rs");
    assert_eq!(all[0].observed, Some(4));
}

#[test]
fn limits_are_inclusive_and_report_the_full_directory_total() {
    let facts = facts(vec![
        file("src/domain/a.rs", SourceKind::Production, 2, false),
        file("src/domain/b.rs", SourceKind::Production, 3, false),
        file("src/domain/c.rs", SourceKind::Production, 7, false),
    ]);

    assert!(evaluate(DirectoryMetric::Files, 3, RuleScope::All, &facts).is_empty());
    assert!(evaluate(DirectoryMetric::CodeLines, 12, RuleScope::All, &facts).is_empty());

    let files = evaluate(DirectoryMetric::Files, 2, RuleScope::All, &facts);
    assert_eq!(files[0].path.as_str(), "src/domain/c.rs");
    assert_eq!(files[0].observed, Some(3));
    assert_eq!(files[0].limit, Some(2));
    assert_eq!(files[0].unit, Some(MetricUnit::Files));

    let lines = evaluate(DirectoryMetric::CodeLines, 4, RuleScope::All, &facts);
    assert_eq!(lines[0].path.as_str(), "src/domain/b.rs");
    assert_eq!(lines[0].observed, Some(12));
    assert_eq!(lines[0].limit, Some(4));
    assert_eq!(lines[0].unit, Some(MetricUnit::CodeLines));
}

#[test]
fn diagnostics_are_deterministic_for_unsorted_facts() {
    let facts = facts(vec![
        file("src/z/b.rs", SourceKind::Production, 2, false),
        file("src/a/c.rs", SourceKind::Production, 2, false),
        file("src/z/a.rs", SourceKind::Production, 2, false),
        file("src/a/b.rs", SourceKind::Production, 2, false),
    ]);

    let diagnostics = evaluate(DirectoryMetric::CodeLines, 2, RuleScope::All, &facts);
    let paths = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, ["src/a/c.rs", "src/z/b.rs"]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.observed == Some(4))
    );
}

#[test]
fn diagnostic_anchor_does_not_split_a_multibyte_first_token() {
    let mut files = ["a", "b", "c", "d", "e"]
        .map(|name| {
            file(
                &format!("src/domain/{name}.rs"),
                SourceKind::Production,
                1,
                false,
            )
        })
        .to_vec();
    files.push(file_with_text(
        "src/domain/z.rs",
        SourceKind::Production,
        1,
        "éclair!();\n",
        false,
    ));
    let facts = facts(files);

    let diagnostics = evaluate(DirectoryMetric::Files, 5, RuleScope::Production, &facts);

    assert_eq!(diagnostics[0].span.byte_start, 0);
    assert_eq!(diagnostics[0].span.byte_end, 0);
}

fn evaluate(
    metric: DirectoryMetric,
    limit: u32,
    scope: RuleScope,
    facts: &CodebaseFacts,
) -> Vec<Diagnostic> {
    let rule = DirectoryLimit {
        config: LimitConfig {
            level: Level::Deny,
            limit,
        },
        metric,
    };
    let mut diagnostics = Vec::new();
    rule.evaluate_scoped(facts, scope, &mut diagnostics);
    diagnostics
}

fn facts(files: Vec<SourceFileFact>) -> CodebaseFacts {
    CodebaseFacts {
        files,
        ..CodebaseFacts::default()
    }
}

fn file(
    relative_path: &str,
    kind: SourceKind,
    code_line_count: u32,
    has_inline_test: bool,
) -> SourceFileFact {
    file_with_text(
        relative_path,
        kind,
        code_line_count,
        "fn item() {}\n",
        has_inline_test,
    )
}

fn file_with_text(
    relative_path: &str,
    kind: SourceKind,
    code_line_count: u32,
    text: &str,
    has_inline_test: bool,
) -> SourceFileFact {
    let source = SourceUnit::new(
        format!("/workspace/{relative_path}").into(),
        relative_path.into(),
        RustEdition::Edition2024,
        text.to_owned(),
    )
    .with_kind(kind);
    let span = source.lines.span(&source.text, 0, source.text.len());
    let test_code = if has_inline_test {
        vec![TestCodeFact {
            kind: TestCodeKind::InlineTestModule,
            name: Some("tests".to_owned()),
            span,
            item_span: span,
        }]
    } else {
        Vec::new()
    };
    SourceFileFact {
        line_count: source.lines.physical_lines(),
        code_line_count,
        functions: Vec::new(),
        test_code,
        source,
    }
}
