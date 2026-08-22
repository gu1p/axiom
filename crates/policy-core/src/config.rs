use std::{collections::BTreeMap, fs};

use camino::Utf8Path;
use serde::Deserialize;
use toml::Table;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    pub version: u32,
    #[serde(default)]
    pub sources: SourceConfig,
    #[serde(default)]
    pub rules: BTreeMap<String, Table>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
    #[serde(default = "default_includes")]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            include: default_includes(),
            exclude: Vec::new(),
        }
    }
}

impl PolicyConfig {
    /// Loads and validates a versioned policy file.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed, or validated.
    pub fn load(path: &Utf8Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?;
        let config: Self = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 {
            return Err(ConfigError::UnsupportedVersion(self.version));
        }
        if self.sources.include.is_empty() {
            return Err(ConfigError::NoIncludes);
        }
        Ok(())
    }
}

fn default_includes() -> Vec<String> {
    vec!["**/*.rs".to_owned()]
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read policy configuration at {path}: {source}")]
    Read {
        path: camino::Utf8PathBuf,
        source: std::io::Error,
    },
    #[error("invalid policy configuration at {path}: {source}")]
    Parse {
        path: camino::Utf8PathBuf,
        source: toml::de::Error,
    },
    #[error("unsupported policy configuration version {0}; expected version 1")]
    UnsupportedVersion(u32),
    #[error("sources.include must contain at least one glob")]
    NoIncludes,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::PolicyConfig;

    fn load(text: &str) -> Result<PolicyConfig, super::ConfigError> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = camino::Utf8PathBuf::from_path_buf(directory.path().join("policy.toml"))
            .expect("UTF-8 path");
        fs::write(&path, text).expect("configuration fixture");
        PolicyConfig::load(&path)
    }

    #[test]
    fn validates_schema_version_and_includes() {
        let version = load("version = 2")
            .expect_err("unsupported version")
            .to_string();
        assert!(version.contains("version 2"));
        let includes = load("version = 1\n[sources]\ninclude = []")
            .expect_err("empty includes")
            .to_string();
        assert!(includes.contains("at least one glob"));
    }

    #[test]
    fn rejects_unknown_top_level_fields() {
        let error = load("version = 1\nfuture = true")
            .expect_err("unknown key")
            .to_string();
        assert!(error.contains("unknown field"));
    }
}
