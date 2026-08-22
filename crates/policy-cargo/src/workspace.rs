use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{MetadataCommand, TargetKind};
use policy_core::{AnalysisError, AnalysisInput, RustEdition, SourceConfig};

use crate::sources::load_sources;

#[derive(Debug, Clone)]
pub(super) struct PackageRoot {
    path: Utf8PathBuf,
    edition: RustEdition,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    root: Utf8PathBuf,
    target_directory: Utf8PathBuf,
    packages: Vec<PackageRoot>,
    has_binary_targets: bool,
}

impl Workspace {
    /// Discovers the containing Cargo workspace and package editions.
    ///
    /// # Errors
    ///
    /// Returns an error when `cargo metadata` cannot describe the workspace.
    pub fn discover(manifest_path: Option<&Utf8Path>) -> Result<Self, WorkspaceError> {
        let mut command = MetadataCommand::new();
        command.no_deps();
        if let Some(path) = manifest_path {
            command.manifest_path(path);
        }
        let metadata = command.exec().map_err(WorkspaceError::Metadata)?;
        let packages = metadata
            .packages
            .iter()
            .filter(|package| metadata.workspace_members.contains(&package.id))
            .filter_map(|package| {
                let path = package.manifest_path.parent()?.to_owned();
                Some(PackageRoot {
                    path,
                    edition: parse_edition(&package.edition.to_string()),
                })
            })
            .collect();
        let has_binary_targets = metadata
            .packages
            .iter()
            .filter(|package| metadata.workspace_members.contains(&package.id))
            .flat_map(|package| &package.targets)
            .any(|target| target.kind.contains(&TargetKind::Bin));
        Ok(Self {
            root: metadata.workspace_root,
            target_directory: metadata.target_directory,
            packages,
            has_binary_targets,
        })
    }

    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn policy_path(&self) -> Utf8PathBuf {
        self.root.join("policy.toml")
    }

    pub fn has_binary_targets(&self) -> bool {
        self.has_binary_targets
    }

    /// Loads every configured Rust source into an analysis input.
    ///
    /// # Errors
    ///
    /// Returns all invalid-glob, traversal, path, UTF-8, and source-reading errors.
    pub fn load(&self, config: &SourceConfig) -> Result<AnalysisInput, Vec<AnalysisError>> {
        let sources = load_sources(&self.root, &self.target_directory, &self.packages, config)?;
        Ok(AnalysisInput {
            workspace_root: self.root.clone(),
            sources,
        })
    }
}

impl PackageRoot {
    pub(super) fn edition_for(path: &Utf8Path, packages: &[Self]) -> RustEdition {
        packages
            .iter()
            .filter(|package| path.starts_with(&package.path))
            .max_by_key(|package| package.path.components().count())
            .map_or(RustEdition::Edition2024, |package| package.edition)
    }
}

fn parse_edition(edition: &str) -> RustEdition {
    match edition {
        "2015" => RustEdition::Edition2015,
        "2018" => RustEdition::Edition2018,
        "2021" => RustEdition::Edition2021,
        _ => RustEdition::Edition2024,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("could not discover Cargo workspace: {0}")]
    Metadata(cargo_metadata::Error),
}

pub(super) type PackageRoots = Vec<PackageRoot>;
