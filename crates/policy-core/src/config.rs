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
    pub semantic: Option<Table>,
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
#[path = "tests/config.rs"]
mod tests;
