use policy_core::{
    AnalysisError, AnalysisInput, CodebaseFacts, FactProvider, FunctionFact, RustEdition,
    SourceFileFact,
};
use ra_ap_syntax::{
    AstNode, AstToken, Edition, SourceFile, SyntaxKind, ast,
    ast::{HasAttrs, HasDocComments, HasName},
};

#[derive(Debug, Default)]
pub struct SyntaxFactProvider;

impl FactProvider for SyntaxFactProvider {
    fn collect(
        &self,
        input: &AnalysisInput,
        facts: &mut CodebaseFacts,
    ) -> Result<(), Vec<AnalysisError>> {
        let mut errors = Vec::new();
        for source in &input.sources {
            let parse = SourceFile::parse(&source.text, map_edition(source.edition));
            for error in parse.errors() {
                let offset: usize = error.range().start().into();
                let span = source.lines.span(&source.text, offset, offset);
                errors.push(
                    AnalysisError::new(format!("could not parse Rust source: {error}"))
                        .at(source.relative_path.clone(), Some(span)),
                );
            }
            if parse.errors().is_empty() {
                let functions = parse
                    .tree()
                    .syntax()
                    .descendants()
                    .filter_map(ast::Fn::cast)
                    .map(|function| function_fact(source, &function))
                    .collect();
                facts.files.push(SourceFileFact {
                    source: source.clone(),
                    line_count: source.lines.physical_lines(),
                    functions,
                });
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn function_fact(source: &policy_core::SourceUnit, function: &ast::Fn) -> FunctionFact {
    let (start, end) = function_bounds(function);
    let (name, name_span) = function.name().map_or_else(
        || {
            (
                "<anonymous>".to_owned(),
                source.lines.span(&source.text, start, start),
            )
        },
        |name| {
            let name_range = name.syntax().text_range();
            let name_start: usize = name_range.start().into();
            let name_end: usize = name_range.end().into();
            (
                name.text().to_string(),
                source.lines.span(&source.text, name_start, name_end),
            )
        },
    );
    FunctionFact {
        name,
        span: source.lines.span(&source.text, start, end),
        name_span,
        line_count: source.lines.span_line_count(start, end),
    }
}

fn function_bounds(function: &ast::Fn) -> (usize, usize) {
    let syntax = function.syntax();
    let (first_code, last_code) = syntax
        .descendants_with_tokens()
        .filter_map(|element| {
            let token = element.into_token()?;
            (!matches!(token.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT))
                .then_some(token.text_range())
        })
        .fold((None, None), |(first, _), range| {
            (first.or(Some(range.start())), Some(range.end()))
        });
    let first_attr = function
        .attrs()
        .next()
        .map(|attr| attr.syntax().text_range().start());
    let first_doc = function
        .doc_comments()
        .next()
        .map(|comment| comment.syntax().text_range().start());
    let raw = syntax.text_range();
    let start = [first_code, first_attr, first_doc]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(raw.start());
    (start.into(), last_code.unwrap_or(raw.end()).into())
}

fn map_edition(edition: RustEdition) -> Edition {
    match edition {
        RustEdition::Edition2015 => Edition::Edition2015,
        RustEdition::Edition2018 => Edition::Edition2018,
        RustEdition::Edition2021 => Edition::Edition2021,
        RustEdition::Edition2024 => Edition::Edition2024,
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use policy_core::{AnalysisInput, CodebaseFacts, FactProvider, RustEdition, SourceUnit};

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
    fn reports_parse_errors_instead_of_partial_facts() {
        let errors = analyze("fn broken( {").expect_err("invalid Rust must fail");
        assert!(!errors.is_empty());
        assert_eq!(
            errors[0].path.as_deref(),
            Some(camino::Utf8Path::new("src/lib.rs"))
        );
    }
}
