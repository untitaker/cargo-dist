//! Forgejo/Codeberg CI script generation

use camino::Utf8PathBuf;
use serde::Serialize;

use crate::{
    config::GithubPermissionMap,
    errors::DistResult,
    DistError, DistGraph,
};

#[cfg(not(windows))]
const FORGEJO_CI_DIR: &str = ".forgejo/workflows/";
#[cfg(windows)]
const FORGEJO_CI_DIR: &str = r".forgejo\workflows\";
const FORGEJO_CI_FILE: &str = "release.yml";

/// Info about running dist in Forgejo CI
#[derive(Debug, Serialize)]
pub struct ForgejoCiInfo {
    /// Cached path to forgejo CI workflows dir
    #[serde(skip_serializing)]
    pub forgejo_ci_workflow_dir: Utf8PathBuf,
    /// Whether to fail-fast
    pub fail_fast: bool,
    /// Whether to cache builds
    pub cache_builds: bool,
}

/// A job in the CI workflow
#[derive(Debug, Default, Clone, Serialize)]
pub struct ForgejoCiJob {
    /// Name of the job
    pub name: String,
    /// Permissions to give the job
    pub permissions: Option<GithubPermissionMap>,
}

impl ForgejoCiInfo {
    /// Load CI script and check that it matches the dist version
    pub fn load_and_check_schema(&self, _dist: &DistGraph) -> DistResult<Option<String>> {
        // For now, always report as missing/needs update
        // TODO: Implement proper schema checking for Forgejo workflows
        Ok(Some("needs update".to_owned()))
    }

    /// Generate the Forgejo workflow file  
    pub fn write_to_disk(&self, _dist: &DistGraph, _dry_run: bool) -> DistResult<()> {
        // TODO: Implement Forgejo workflow generation
        Ok(())
    }
}

/// Generate Forgejo CI for the given dist graph
pub fn generate_forgejo_ci(dist: &DistGraph) -> DistResult<ForgejoCiInfo> {
    let config = dist
        .config
        .ci
        .forgejo
        .as_ref()
        .ok_or_else(|| DistError::UnrecognizedCiStyle { style: "forgejo".to_owned() })?;

    let workflow_dir = dist.workspace_dir.join(FORGEJO_CI_DIR);

    // Build basic info structure
    Ok(ForgejoCiInfo {
        forgejo_ci_workflow_dir: workflow_dir,
        fail_fast: config.common.fail_fast,
        cache_builds: config.common.cache_builds.unwrap_or(false),
    })
}