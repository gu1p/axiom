use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolConfig {
    pub clippy: ClippyConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ClippyConfig {
    pub enabled: bool,
    pub profile: ClippyLintProfile,
    pub check_docs: bool,
    pub targets: ClippyTargetCoverage,
    pub features: ClippyFeatureSelection,
    pub no_default_features: bool,
    pub warnings: ClippyWarningPolicy,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClippyLintProfile {
    #[default]
    Axiom,
    Workspace,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClippyTargetCoverage {
    #[default]
    All,
    Default,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(untagged)]
pub enum ClippyFeatureSelection {
    #[default]
    Default,
    Mode(ClippyFeatureMode),
    Selected(Vec<String>),
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClippyFeatureMode {
    Default,
    All,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClippyWarningPolicy {
    #[default]
    Deny,
    Warn,
}

impl Default for ClippyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            profile: ClippyLintProfile::Axiom,
            check_docs: true,
            targets: ClippyTargetCoverage::All,
            features: ClippyFeatureSelection::Default,
            no_default_features: false,
            warnings: ClippyWarningPolicy::Deny,
        }
    }
}

impl ClippyConfig {
    #[must_use]
    pub fn uses_axiom_profile(&self) -> bool {
        matches!(self.profile, ClippyLintProfile::Axiom)
    }

    #[must_use]
    pub fn checks_all_targets(&self) -> bool {
        matches!(self.targets, ClippyTargetCoverage::All)
    }

    #[must_use]
    pub fn checks_all_features(&self) -> bool {
        matches!(
            self.features,
            ClippyFeatureSelection::Mode(ClippyFeatureMode::All)
        )
    }

    #[must_use]
    pub fn selected_features(&self) -> Option<&[String]> {
        match &self.features {
            ClippyFeatureSelection::Selected(features) if !features.is_empty() => Some(features),
            _ => None,
        }
    }

    #[must_use]
    pub fn denies_warnings(&self) -> bool {
        matches!(self.warnings, ClippyWarningPolicy::Deny)
    }
}
