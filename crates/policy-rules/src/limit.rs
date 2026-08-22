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
mod tests {
    use toml::Table;

    use super::LimitConfig;

    fn table(text: &str) -> Table {
        toml::from_str(text).expect("valid TOML table")
    }

    #[test]
    fn accepts_allow_warn_and_deny_levels() {
        for level in ["allow", "warn", "deny"] {
            let text = format!("level = \"{level}\"\nlimit = 1");
            assert!(LimitConfig::parse("example", &table(&text)).is_ok());
        }
    }

    #[test]
    fn rejects_zero_limits_and_unknown_options() {
        let zero = LimitConfig::parse("example", &table("level = \"deny\"\nlimit = 0"));
        assert!(
            zero.expect_err("zero must fail")
                .contains("greater than zero")
        );

        let unknown = LimitConfig::parse(
            "example",
            &table("level = \"deny\"\nlimit = 1\nfuture = true"),
        );
        assert!(
            unknown
                .expect_err("unknown key must fail")
                .contains("unknown field")
        );
    }
}
