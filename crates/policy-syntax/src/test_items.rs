use policy_core::{SourceUnit, TestCodeFact, TestCodeKind};
use ra_ap_syntax::{
    AstNode,
    ast::{self, HasAttrs, HasName},
};

pub fn collect(source: &SourceUnit, root: &ra_ap_syntax::SyntaxNode) -> Vec<TestCodeFact> {
    root.descendants()
        .filter_map(ast::Item::cast)
        .filter_map(|item| test_code_fact(source, &item))
        .collect()
}

fn test_code_fact(source: &SourceUnit, item: &ast::Item) -> Option<TestCodeFact> {
    if has_test_only_ancestor(item) {
        return None;
    }
    if let Some(attribute) = item.attrs().find(is_test_only_cfg) {
        if is_external_module(item) {
            return None;
        }
        let kind = if matches!(item, ast::Item::Module(_)) {
            TestCodeKind::InlineTestModule
        } else {
            TestCodeKind::TestOnlyItem
        };
        return Some(fact(source, item, &attribute, kind));
    }
    let ast::Item::Fn(function) = item else {
        return None;
    };
    function
        .attrs()
        .find(is_test_attribute)
        .map(|attribute| fact(source, item, &attribute, TestCodeKind::TestFunction))
}

fn fact(
    source: &SourceUnit,
    item: &ast::Item,
    attribute: &ast::Attr,
    kind: TestCodeKind,
) -> TestCodeFact {
    let range = attribute.syntax().text_range();
    TestCodeFact {
        kind,
        name: item_name(item),
        span: source.lines.span(
            &source.text,
            usize::from(range.start()),
            usize::from(range.end()),
        ),
    }
}

fn item_name(item: &ast::Item) -> Option<String> {
    match item {
        ast::Item::Fn(item) => item.name(),
        ast::Item::Module(item) => item.name(),
        _ => None,
    }
    .map(|name| name.text().to_string())
}

fn has_test_only_ancestor(item: &ast::Item) -> bool {
    item.syntax()
        .ancestors()
        .skip(1)
        .filter_map(ast::Item::cast)
        .any(|ancestor| {
            ancestor
                .attrs()
                .any(|attribute| is_test_only_cfg(&attribute))
        })
}

fn is_external_module(item: &ast::Item) -> bool {
    matches!(item, ast::Item::Module(module) if module.item_list().is_none())
}

fn is_test_only_cfg(attribute: &ast::Attr) -> bool {
    let Some(ast::Meta::CfgMeta(meta)) = attribute.meta() else {
        return false;
    };
    meta.cfg_predicate()
        .is_some_and(|predicate| !predicate_values(&predicate, false).can_true)
}

fn is_test_attribute(attribute: &ast::Attr) -> bool {
    attribute.meta().is_some_and(|meta| match meta {
        ast::Meta::CfgAttrMeta(cfg_attr) => {
            let enabled_for_tests = cfg_attr
                .cfg_predicate()
                .is_some_and(|predicate| predicate_values(&predicate, true).can_true);
            enabled_for_tests && cfg_attr.metas().any(|meta| meta_is_test_attribute(&meta))
        }
        _ => meta_is_test_attribute(&meta),
    })
}

fn meta_is_test_attribute(meta: &ast::Meta) -> bool {
    let Some(path) = meta.path() else {
        return false;
    };
    let text = path.syntax().text().to_string();
    matches!(
        text.rsplit("::").next().map(str::trim),
        Some("test" | "bench" | "rstest" | "test_case")
    )
}

#[derive(Clone, Copy)]
struct PredicateValues {
    can_false: bool,
    can_true: bool,
}

fn predicate_values(predicate: &ast::CfgPredicate, test: bool) -> PredicateValues {
    match predicate {
        ast::CfgPredicate::CfgAtom(atom) => atom_values(atom, test),
        ast::CfgPredicate::CfgComposite(composite) => composite_values(composite, test),
    }
}

fn atom_values(atom: &ast::CfgAtom, test: bool) -> PredicateValues {
    if atom.eq_token().is_none()
        && atom
            .ident_token()
            .is_some_and(|token| token.text() == "test")
    {
        return PredicateValues {
            can_false: !test,
            can_true: test,
        };
    }
    if atom.true_token().is_some() {
        return PredicateValues {
            can_false: false,
            can_true: true,
        };
    }
    if atom.false_token().is_some() {
        return PredicateValues {
            can_false: true,
            can_true: false,
        };
    }
    PredicateValues {
        can_false: true,
        can_true: true,
    }
}

fn composite_values(composite: &ast::CfgComposite, test: bool) -> PredicateValues {
    let values: Vec<_> = composite
        .cfg_predicates()
        .map(|predicate| predicate_values(&predicate, test))
        .collect();
    match composite
        .keyword()
        .as_ref()
        .map(ra_ap_syntax::SyntaxToken::text)
    {
        Some("all") => PredicateValues {
            can_false: values.iter().any(|value| value.can_false),
            can_true: values.iter().all(|value| value.can_true),
        },
        Some("any") => PredicateValues {
            can_false: values.iter().all(|value| value.can_false),
            can_true: values.iter().any(|value| value.can_true),
        },
        Some("not") if values.len() == 1 => PredicateValues {
            can_false: values[0].can_true,
            can_true: values[0].can_false,
        },
        _ => PredicateValues {
            can_false: true,
            can_true: true,
        },
    }
}
