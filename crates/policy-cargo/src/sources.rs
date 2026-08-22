use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::{DirEntry, WalkBuilder};
use policy_core::{AnalysisError, SourceConfig, SourceUnit};

use crate::workspace::{PackageRoot, PackageRoots};

pub(super) fn load_sources(
    root: &Utf8Path,
    target: &Utf8Path,
    packages: &PackageRoots,
    config: &SourceConfig,
) -> Result<Vec<SourceUnit>, Vec<AnalysisError>> {
    let include = build_globs(&config.include).map_err(|error| vec![error])?;
    let exclude = build_globs(&config.exclude).map_err(|error| vec![error])?;
    let entries = walk(root, target);
    let mut sources = Vec::new();
    let mut errors = Vec::new();

    for result in entries {
        match result {
            Ok(entry) => load_entry(
                entry,
                root,
                packages,
                &include,
                &exclude,
                &mut sources,
                &mut errors,
            ),
            Err(error) => errors.push(AnalysisError::new(format!(
                "source discovery failed: {error}"
            ))),
        }
    }
    if errors.is_empty() {
        sources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(sources)
    } else {
        Err(errors)
    }
}

fn walk(root: &Utf8Path, target: &Utf8Path) -> ignore::Walk {
    let root_path = root.as_std_path().to_owned();
    let target_path = target.as_std_path().to_owned();
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            let path = entry.path();
            if path == target_path || path.starts_with(&target_path) {
                return false;
            }
            path.strip_prefix(&root_path)
                .ok()
                .is_none_or(|relative| !relative.iter().any(|part| part == ".git"))
        });
    builder.build()
}

#[allow(clippy::too_many_arguments)]
fn load_entry(
    entry: DirEntry,
    root: &Utf8Path,
    packages: &PackageRoots,
    include: &GlobSet,
    exclude: &GlobSet,
    sources: &mut Vec<SourceUnit>,
    errors: &mut Vec<AnalysisError>,
) {
    if !entry.file_type().is_some_and(|kind| kind.is_file()) {
        return;
    }
    let Ok(absolute) = Utf8PathBuf::from_path_buf(entry.into_path()) else {
        errors.push(AnalysisError::new("Rust source path is not valid UTF-8"));
        return;
    };
    let Some(relative) = absolute.strip_prefix(root).ok().map(Utf8Path::to_owned) else {
        return;
    };
    let normalized = relative.as_str().replace('\\', "/");
    if !include.is_match(&normalized) || exclude.is_match(&normalized) {
        return;
    }
    match fs::read_to_string(&absolute) {
        Ok(text) => sources.push(SourceUnit::new(
            absolute.clone(),
            relative,
            PackageRoot::edition_for(&absolute, packages),
            text,
        )),
        Err(error) => errors.push(
            AnalysisError::new(format!("could not read Rust source: {error}")).at(relative, None),
        ),
    }
}

fn build_globs(patterns: &[String]) -> Result<GlobSet, AnalysisError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|error| {
            AnalysisError::new(format!("invalid source glob `{pattern}`: {error}"))
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|error| AnalysisError::new(format!("could not build source globs: {error}")))
}
