use policy_core::{ClippyConfig, Level};

const RUST_WARN: &[&str] = &[
    "future-incompatible",
    "nonstandard-style",
    "rust-2018-idioms",
    "trivial-numeric-casts",
    "unused-import-braces",
    "unused-lifetimes",
];

const RUST_ALLOW: &[&str] = &["trivial-casts", "unused-qualifications"];
const CLIPPY_GROUPS: &[&str] = &["all", "cargo", "pedantic"];

const CLIPPY_WARN: &[&str] = &[
    "allow-attributes",
    "as-ptr-cast-mut",
    "branches-sharing-code",
    "clear-with-drain",
    "clone-on-ref-ptr",
    "coerce-container-to-any",
    "dbg-macro",
    "debug-assert-with-mut-call",
    "default-union-representation",
    "derive-partial-eq-without-eq",
    "disallowed-script-idents",
    "doc-include-without-cfg",
    "empty-enum-variants-with-brackets",
    "equatable-if-let",
    "exit",
    "fallible-impl-from",
    "float-cmp-const",
    "fn-to-numeric-cast-any",
    "get-unwrap",
    "imprecise-flops",
    "infinite-loop",
    "iter-on-empty-collections",
    "iter-on-single-items",
    "iter-over-hash-type",
    "large-include-file",
    "large-stack-frames",
    "literal-string-with-formatting-args",
    "lossy-float-literal",
    "map-err-ignore",
    "mem-forget",
    "missing-assert-message",
    "mutex-integer",
    "needless-pass-by-ref-mut",
    "needless-type-cast",
    "non-zero-suggestions",
    "nonstandard-macro-braces",
    "or-fun-call",
    "path-buf-push-overwrite",
    "pathbuf-init-then-push",
    "precedence-bits",
    "print-stderr",
    "print-stdout",
    "pub-without-shorthand",
    "rc-mutex",
    "redundant-type-annotations",
    "ref-patterns",
    "rest-pat-in-fully-bound-structs",
    "return-and-then",
    "set-contains-or-insert",
    "single-option-map",
    "std-instead-of-core",
    "str-to-string",
    "string-add",
    "string-lit-as-bytes",
    "string-lit-chars-any",
    "suspicious-xor-used-as-pow",
    "todo",
    "too-long-first-doc-paragraph",
    "trailing-empty-array",
    "trait-duplication-in-bounds",
    "tuple-array-conversions",
    "undocumented-unsafe-blocks",
    "unimplemented",
    "uninhabited-references",
    "unnecessary-safety-comment",
    "unnecessary-safety-doc",
    "unnecessary-self-imports",
    "unnecessary-struct-initialization",
    "unused-peekable",
    "unused-rounding",
    "unused-trait-names",
    "unwrap-used",
    "use-self",
    "useless-let-if-seq",
    "verbose-file-reads",
];

const CLIPPY_ALLOW: &[&str] = &[
    "assigning-clones",
    "cast-lossless",
    "cast-possible-truncation",
    "cast-possible-wrap",
    "cast-precision-loss",
    "cast-sign-loss",
    "comparison-chain",
    "default-trait-access",
    "float-cmp",
    "inline-always",
    "items-after-statements",
    "let-underscore-must-use",
    "let-underscore-untyped",
    "manual-range-contains",
    "many-single-char-names",
    "map-unwrap-or",
    "missing-panics-doc",
    "multiple-crate-versions",
    "must-use-candidate",
    "redundant-closure-for-method-calls",
    "return-self-not-must-use",
    "self-named-module-files",
    "should-panic-without-expect",
    "significant-drop-tightening",
    "similar-names",
    "struct-excessive-bools",
    "struct-field-names",
    "too-many-lines",
    "trivially-copy-pass-by-ref",
    "unreadable-literal",
    "used-underscore-binding",
    "wildcard-imports",
];

pub fn compiler_arguments(config: &ClippyConfig) -> Vec<String> {
    let mut arguments = Vec::new();
    if config.uses_axiom_profile() {
        extend(&mut arguments, "-W", None, RUST_WARN);
        extend(&mut arguments, "-W", Some("clippy"), CLIPPY_GROUPS);
        extend(&mut arguments, "-W", Some("clippy"), CLIPPY_WARN);
    }
    if config.denies_warnings() {
        push(&mut arguments, "-D", "warnings");
    }
    if config.uses_axiom_profile() {
        push(&mut arguments, "-D", "unsafe-code");
        extend(&mut arguments, "-A", None, RUST_ALLOW);
        extend(&mut arguments, "-A", Some("clippy"), CLIPPY_ALLOW);
    }
    for (lint, level) in config
        .lint_overrides()
        .filter(|(lint, _)| !lint.starts_with("rustdoc::"))
    {
        push(&mut arguments, level_flag(level), lint);
    }
    arguments
}

pub fn rustdoc_arguments(config: &ClippyConfig) -> Vec<String> {
    let mut arguments = Vec::new();
    if config.uses_axiom_profile() {
        push(&mut arguments, "-W", "rustdoc::all");
    }
    if config.denies_warnings() {
        push(&mut arguments, "-D", "warnings");
    }
    for (lint, level) in config
        .lint_overrides()
        .filter(|(lint, _)| !lint.starts_with("clippy::"))
    {
        push(&mut arguments, level_flag(level), lint);
    }
    arguments
}

const fn level_flag(level: Level) -> &'static str {
    match level {
        Level::Allow => "-A",
        Level::Warn => "--force-warn",
        Level::Deny => "-D",
    }
}

fn extend(arguments: &mut Vec<String>, level: &str, namespace: Option<&str>, lints: &[&str]) {
    for lint in lints {
        let name =
            namespace.map_or_else(|| (*lint).to_owned(), |prefix| format!("{prefix}::{lint}"));
        push(arguments, level, &name);
    }
}

fn push(arguments: &mut Vec<String>, level: &str, lint: &str) {
    arguments.push(level.to_owned());
    arguments.push(lint.to_owned());
}
