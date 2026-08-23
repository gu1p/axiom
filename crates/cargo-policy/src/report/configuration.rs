use camino::{Utf8Path, Utf8PathBuf};
use policy_core::{Diagnostic, Level};
use serde::Serialize;

use crate::tools::ToolDiagnostic;

use super::output::stderr_line;

const LEVEL_HELP: &str = "\"deny\" = error, \"warn\" = warning, \"allow\" = disabled";

pub(super) struct Hint {
    pub key: String,
    pub level: &'static str,
}

#[derive(Serialize)]
pub(super) struct JsonHint {
    file: Utf8PathBuf,
    key: String,
    value: &'static str,
    levels: LevelMeanings,
}

#[derive(Serialize)]
struct LevelMeanings {
    deny: &'static str,
    warn: &'static str,
    allow: &'static str,
}

pub(super) fn policy(diagnostic: &Diagnostic) -> Hint {
    Hint {
        key: format!("rules.{}.level", quoted(&diagnostic.rule_id)),
        level: level_name(diagnostic.level),
    }
}

pub(super) fn tool(diagnostic: &ToolDiagnostic) -> Option<Hint> {
    if !matches!(diagnostic.tool, "clippy" | "rustdoc")
        || diagnostic.rule_id == diagnostic.tool
        || is_compiler_error_code(&diagnostic.rule_id)
    {
        return None;
    }
    Some(Hint {
        key: format!("tools.clippy.lints.{}", quoted(&diagnostic.rule_id)),
        level: level_name(diagnostic.level),
    })
}

pub(super) fn write_human(path: &Utf8Path, hint: &Hint) {
    stderr_line(format_args!(
        "  = policy: {} = \"{}\" in {path}",
        hint.key, hint.level
    ));
    stderr_line(format_args!("  = configure: {LEVEL_HELP}"));
}

pub(super) fn json(path: &Utf8Path, hint: Hint) -> JsonHint {
    JsonHint {
        file: path.to_owned(),
        key: hint.key,
        value: hint.level,
        levels: LevelMeanings {
            deny: "error",
            warn: "warning",
            allow: "disabled",
        },
    }
}

fn quoted(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

const fn level_name(level: Level) -> &'static str {
    match level {
        Level::Allow => "allow",
        Level::Warn => "warn",
        Level::Deny => "deny",
    }
}

fn is_compiler_error_code(code: &str) -> bool {
    code.strip_prefix('E').is_some_and(|digits| {
        digits.len() == 4 && digits.bytes().all(|digit| digit.is_ascii_digit())
    })
}
