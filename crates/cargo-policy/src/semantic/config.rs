use std::io::Write as _;

use policy_core::AnalysisError;
use toml::{Table, Value};

pub(super) fn semantic_config_file(
    config: Option<&Table>,
) -> Result<(Option<tempfile::NamedTempFile>, Vec<String>), AnalysisError> {
    let Some(config) = config else {
        return Ok((None, Vec::new()));
    };
    let mut config = config.clone();
    let excluded_crates = take_string_array(&mut config, "exclude-crates")?;
    normalize_overrides(&mut config)?;
    let encoded = toml::to_string(&config).map_err(|error| {
        AnalysisError::new(format!(
            "could not encode [semantic] configuration: {error}"
        ))
    })?;
    let mut file = tempfile::Builder::new()
        .prefix("axiom-semantic-")
        .suffix(".toml")
        .tempfile()
        .map_err(|error| {
            AnalysisError::new(format!("could not create semantic config: {error}"))
        })?;
    file.write_all(encoded.as_bytes())
        .map_err(|error| AnalysisError::new(format!("could not write semantic config: {error}")))?;
    Ok((Some(file), excluded_crates))
}

fn take_string_array(table: &mut Table, key: &str) -> Result<Vec<String>, AnalysisError> {
    let Some(value) = table.remove(key) else {
        return Ok(Vec::new());
    };
    let Value::Array(values) = value else {
        return Err(AnalysisError::new(format!(
            "semantic.{key} must be an array of strings"
        )));
    };
    values
        .into_iter()
        .map(|value| match value {
            Value::String(value) => Ok(value),
            _ => Err(AnalysisError::new(format!(
                "semantic.{key} must contain only strings"
            ))),
        })
        .collect()
}

pub(super) fn normalize_overrides(table: &mut Table) -> Result<(), AnalysisError> {
    let Some(Value::Array(overrides)) = table.get_mut("override") else {
        return Ok(());
    };
    for entry in overrides {
        let Value::Table(entry) = entry else {
            continue;
        };
        if let Some(rule) = entry.remove("rule") {
            let Value::String(rule) = rule else {
                return Err(AnalysisError::new(
                    "semantic.override.rule must be a string",
                ));
            };
            let lint = axiom_rule_to_hawk(&rule).ok_or_else(|| {
                AnalysisError::new(format!(
                    "semantic override references unsupported rule `{rule}`"
                ))
            })?;
            entry.insert("lint".to_owned(), Value::String(lint.to_owned()));
        }
    }
    Ok(())
}

pub(super) fn axiom_rule_to_hawk(rule: &str) -> Option<&'static str> {
    match rule {
        "dead-code/public" => Some("hawk::dead_public"),
        "dead-code/test-only" => Some("hawk::test_only"),
        "visibility/unnecessary-public" => Some("hawk::unnecessary_public"),
        "visibility/unnecessary-restricted" => Some("hawk::unnecessary_restricted_visibility"),
        "visibility/unnecessary-crate" => Some("hawk::unnecessary_crate_visibility"),
        _ => None,
    }
}
