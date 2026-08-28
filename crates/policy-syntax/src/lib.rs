//! Lossless Rust syntax facts used by Axiom policies.

mod test_items;

use std::collections::BTreeSet;

use policy_core::{
    AnalysisError, AnalysisInput, CodebaseFacts, FactProvider, FunctionFact, RustEdition,
    SourceFileFact,
};
use ra_ap_syntax::{
    AstNode as _, AstToken as _, Edition, SourceFile, SyntaxKind, ast,
    ast::{HasAttrs as _, HasDocComments as _, HasName as _},
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
                let tree = parse.tree();
                let functions = tree
                    .syntax()
                    .descendants()
                    .filter_map(ast::Fn::cast)
                    .map(|function| function_fact(source, &function))
                    .collect();
                facts.files.push(SourceFileFact {
                    source: source.clone(),
                    line_count: source.lines.physical_lines(),
                    code_line_count: code_line_count(source, tree.syntax()),
                    functions,
                    test_code: test_items::collect(source, tree.syntax()),
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

fn code_line_count(source: &policy_core::SourceUnit, syntax: &ra_ap_syntax::SyntaxNode) -> u32 {
    let mut code_lines = BTreeSet::new();
    for token in syntax
        .descendants_with_tokens()
        .filter_map(ra_ap_syntax::NodeOrToken::into_token)
        .filter(|token| !matches!(token.kind(), SyntaxKind::WHITESPACE | SyntaxKind::COMMENT))
    {
        let range = token.text_range();
        let start: usize = range.start().into();
        let Some((last_offset, _)) = token.text().char_indices().next_back() else {
            continue;
        };
        let span = source.lines.span(&source.text, start, start + last_offset);
        code_lines.extend(span.start.line..=span.end.line);
    }
    code_lines.len().try_into().unwrap_or(u32::MAX)
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
            (first.or_else(|| Some(range.start())), Some(range.end()))
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
        .unwrap_or_else(|| raw.start());
    (start.into(), last_code.unwrap_or_else(|| raw.end()).into())
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
#[path = "tests/syntax.rs"]
mod tests;
