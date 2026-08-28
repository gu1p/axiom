use camino::Utf8PathBuf;
use policy_core::{
    AnalysisInput, CodebaseFacts, FactProvider as _, RustEdition, SourceUnit, TestCodeKind,
};

use super::SyntaxFactProvider;

fn analyze(text: &str) -> Result<CodebaseFacts, Vec<policy_core::AnalysisError>> {
    let source = SourceUnit::new(
        Utf8PathBuf::from("/workspace/src/lib.rs"),
        Utf8PathBuf::from("src/lib.rs"),
        RustEdition::Edition2024,
        text.to_owned(),
    );
    let input = AnalysisInput {
        workspace_root: Utf8PathBuf::from("/workspace"),
        sources: vec![source],
    };
    let mut facts = CodebaseFacts::default();
    SyntaxFactProvider.collect(&input, &mut facts)?;
    Ok(facts)
}

#[test]
fn includes_attributes_signatures_blanks_and_comments() {
    let text =
        "#[test]\nfn example(\n    value: u32,\n) -> u32 {\n\n    // retained\n    value\n}\n";
    let facts = analyze(text).expect("valid Rust");
    let function = &facts.files[0].functions[0];
    assert_eq!(function.name, "example");
    assert_eq!(function.line_count, 8);
    assert_eq!(function.name_span.start.line, 2);
}

#[test]
fn includes_docs_but_excludes_unattached_leading_comments() {
    let text = "// unrelated\n\n/// attached\nfn example() {}\n";
    let facts = analyze(text).expect("valid Rust");
    let function = &facts.files[0].functions[0];
    assert_eq!(function.line_count, 2);
    assert_eq!(function.span.start.line, 3);
}

#[test]
fn counts_only_lines_with_non_trivia_tokens() {
    let text = r"
// ordinary comment
/// documentation comment
fn example() { // mixed code and comment
    /* multiline
       comment */
    let value = 1;

    value
}
";
    let facts = analyze(text).expect("valid Rust");
    assert_eq!(facts.files[0].line_count, 10);
    assert_eq!(facts.files[0].code_line_count, 4);
}

#[test]
fn counts_every_physical_line_touched_by_a_multiline_code_token() {
    let text = "const VALUE: &str = r#\"first\nsecond\nthird\"#;\n";
    let facts = analyze(text).expect("valid Rust");
    assert_eq!(facts.files[0].code_line_count, 3);
}

#[test]
fn includes_inline_test_code_in_the_file_count() {
    let text = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n";
    let facts = analyze(text).expect("valid Rust");
    assert_eq!(facts.files[0].code_line_count, 5);
}

#[test]
fn finds_free_trait_impl_extern_and_nested_functions() {
    let text = r#"
fn free() { fn nested() {} }
trait Work { fn required(); fn provided() {} }
impl Work for () { fn required() {} }
extern "C" { fn external(); }
"#;
    let facts = analyze(text).expect("valid Rust");
    let names: Vec<_> = facts.files[0]
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "free", "nested", "required", "provided", "required", "external"
        ]
    );
}

#[test]
fn identifies_inline_test_modules_and_test_functions_once() {
    let text = r"
#[cfg(test)]
mod tests {
    #[test]
    fn nested_test() {}
}

#[tokio::test]
async fn async_test() {}
";
    let facts = analyze(text).expect("valid Rust");
    let test_code = &facts.files[0].test_code;
    assert_eq!(test_code.len(), 2);
    assert_eq!(test_code[0].kind, TestCodeKind::InlineTestModule);
    assert_eq!(test_code[0].name.as_deref(), Some("tests"));
    assert_eq!(test_code[1].kind, TestCodeKind::TestFunction);
    assert_eq!(test_code[1].name.as_deref(), Some("async_test"));
}

#[test]
fn allows_external_test_modules_and_non_test_cfg_items() {
    let text = r"
#[cfg(test)]
mod tests;

#[cfg(not(test))]
fn production_only() {}
";
    let facts = analyze(text).expect("valid Rust");
    assert!(facts.files[0].test_code.is_empty());
}

#[test]
fn reports_parse_errors_instead_of_partial_facts() {
    let errors = analyze("fn broken( {").expect_err("invalid Rust must fail");
    assert!(!errors.is_empty());
    assert_eq!(
        errors[0].path.as_deref(),
        Some(camino::Utf8Path::new("src/lib.rs"))
    );
}
