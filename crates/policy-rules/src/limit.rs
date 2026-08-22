use policy_core::Level;
use serde::Deserialize;
use toml::Table;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitConfig {
    pub level: Level,
    pub limit: u32,
}

impl LimitConfig {
    pub fn parse(id: &str, table: &Table) -> Result<Self, String> {
        let config: Self = toml::Value::Table(table.clone())
            .try_into()
            .map_err(|error| format!("invalid configuration for `{id}`: {error}"))?;
        if config.limit == 0 {
            return Err(format!(
                "invalid configuration for `{id}`: limit must be greater than zero"
            ));
        }
        Ok(config)
    }
}

#[cfg(test)]
#[path = "tests/limit.rs"]
mod tests;
