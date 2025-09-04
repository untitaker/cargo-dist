//! Forgejo/Codeberg CI script generation

use camino::Utf8PathBuf;
use dist_schema::{GithubRunnerRef, TripleNameRef};
use serde::Serialize;

use crate::{
    backend::{diff_files, templates::TEMPLATE_CI_FORGEJO},
    config::GithubPermissionMap,
    errors::DistResult,
    DistError, DistGraph,
};
use axoasset::LocalAsset;

use super::{DistInstallSettings, InstallStrategy};

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
    /// Whether to include builtin local artifacts tasks
    pub build_local_artifacts: bool,
    /// Whether to make CI get dispatched manually instead of by tag
    pub dispatch_releases: bool,
    /// Trigger releases on pushes to this branch instead of tags
    pub release_branch: Option<String>,
    /// Matrix for upload-local-artifacts
    pub artifacts_matrix: dist_schema::GithubMatrix,
    /// What kind of job to run on pull request
    pub pr_run_mode: dist_schema::PrRunMode,
    /// global task
    pub global_task: dist_schema::GithubGlobalJobConfig,
    /// plan jobs
    pub plan_jobs: Vec<ForgejoCiJob>,
    /// local artifacts jobs
    pub local_artifacts_jobs: Vec<ForgejoCiJob>,
    /// global artifacts jobs
    pub global_artifacts_jobs: Vec<ForgejoCiJob>,
    /// host jobs
    pub host_jobs: Vec<ForgejoCiJob>,
    /// publish jobs
    pub publish_jobs: Vec<String>,
    /// user-specified publish jobs
    pub user_publish_jobs: Vec<ForgejoCiJob>,
    /// post-announce jobs
    pub post_announce_jobs: Vec<ForgejoCiJob>,
    /// what hosting provider we're using
    pub hosting_providers: Vec<crate::config::HostingStyle>,
    /// whether to prefix release.yml and the tag pattern
    pub tag_namespace: Option<String>,
    /// Extra permissions the workflow file should have
    pub root_permissions: Option<GithubPermissionMap>,
    /// Whether to create the release or assume an existing one
    pub create_release: bool,
    /// How to install dist when "coordinating" (plan, global build, etc.)
    pub dist_install_for_coordinator: dist_schema::GhaRunStep,
    /// Custom build setup script
    pub build_setup: Option<String>,
}

/// A job in the CI workflow
#[derive(Debug, Default, Clone, Serialize)]
pub struct ForgejoCiJob {
    /// ID of the job
    pub id: String,
    /// Name of the job
    pub name: String,
    /// Runner to use for this job
    pub runner: String,
    /// Whether this job should only run on releases
    pub only_if_release: Option<bool>,
    /// Permissions to give the job
    pub permissions: Option<GithubPermissionMap>,
}

impl ForgejoCiInfo {
    /// Get the path for the forgejo release.yml file
    pub fn forgejo_ci_release_yml_path(&self) -> Utf8PathBuf {
        let prefix = "";  // TODO: Add tag namespace support like GitHub
        self.forgejo_ci_workflow_dir
            .join(format!("{prefix}{FORGEJO_CI_FILE}"))
    }

    /// Generate the requested configuration and returns it as a string.
    pub fn generate_forgejo_ci(&self, dist: &DistGraph) -> DistResult<String> {
        let rendered = dist
            .templates
            .render_file_to_clean_string(TEMPLATE_CI_FORGEJO, self)?;
        Ok(rendered)
    }

    /// Write release.yml to disk
    pub fn write_to_disk(&self, dist: &DistGraph, dry_run: bool) -> DistResult<()> {
        let ci_file = self.forgejo_ci_release_yml_path();
        let rendered = self.generate_forgejo_ci(dist)?;
        if dry_run {
            // For dry run, just check if content would change
            return self.check(dist).map_err(|_| ()).or(Ok(()));
        }
        LocalAsset::write_new_all(&rendered, &ci_file)?;
        eprintln!("generated Forgejo CI to {}", ci_file);
        Ok(())
    }

    /// Check whether the new configuration differs from the config on disk
    /// without actually writing the result.
    pub fn check(&self, dist: &DistGraph) -> DistResult<()> {
        let ci_file = self.forgejo_ci_release_yml_path();
        let rendered = self.generate_forgejo_ci(dist)?;
        diff_files(&ci_file, &rendered)
    }

    /// Load CI script and check that it matches the dist version
    pub fn load_and_check_schema(&self, dist: &DistGraph) -> DistResult<Option<String>> {
        match self.check(dist) {
            Ok(()) => Ok(None), // Files match, no update needed
            Err(_) => Ok(Some("needs update".to_owned())), // Files differ or missing
        }
    }
}

/// Generate Forgejo CI for the given dist graph
pub fn generate_forgejo_ci(dist: &DistGraph) -> DistResult<ForgejoCiInfo> {
    eprintln!("DEBUG: generate_forgejo_ci called");
    let config = dist
        .config
        .ci
        .forgejo
        .as_ref()
        .ok_or_else(|| DistError::UnrecognizedCiStyle { style: "forgejo".to_owned() })?;

    let workflow_dir = dist.workspace_dir.join(FORGEJO_CI_DIR);

    // If they don't specify a dist version, use this one
    let self_dist_version = super::SELF_DIST_VERSION.parse().unwrap();
    let dist_version = dist
        .config
        .dist_version
        .as_ref()
        .unwrap_or(&self_dist_version);

    let fail_fast = config.common.fail_fast;
    let build_local_artifacts = config.common.build_local_artifacts;
    let dispatch_releases = config.common.dispatch_releases;
    let release_branch = config.common.release_branch.clone();
    let tag_namespace = config.common.tag_namespace.clone();
    let pr_run_mode = config.common.pr_run_mode;

    let caching_could_be_profitable =
        release_branch.is_some() || pr_run_mode == dist_schema::PrRunMode::Upload;
    let cache_builds = config.common.cache_builds.unwrap_or(caching_could_be_profitable);

    // Get the repository URL from hosting info  
    // Note: h.repo_path already starts with "/" so we don't need additional separator
    let repository_url = dist.hosting.as_ref().map(|h| {
        format!("{}{}", h.domain, h.repo_path)
    });
    
    let dist_install_strategy = (DistInstallSettings {
        version: dist_version,
        url_override: dist.config.dist_url_override.as_deref(),
        repository_url: repository_url.clone(),
    })
    .install_strategy();

    let hosting_providers = dist
        .hosting
        .as_ref()
        .map(|h| h.hosts.clone())
        .unwrap_or_else(|| vec![crate::config::HostingStyle::Forgejo]);

    // Create default global runner config - using x86_64-unknown-linux-gnu as default for ubuntu-22.04
    let global_runner = GithubRunnerRef::from_str("ubuntu-22.04").to_owned();
    let host_triple = TripleNameRef::from_str("x86_64-unknown-linux-gnu").to_owned();
    let global_runner_config = dist_schema::GithubRunnerConfig {
        runner: global_runner,
        host: host_triple,
        container: None,
    };
    let global_task = dist_schema::GithubGlobalJobConfig {
        runner: global_runner_config,
        install_dist: dist_install_strategy.for_triple(&dist_schema::target_lexicon::Triple::host()),
        dist_args: String::new(),
        install_cargo_cyclonedx: None,
        install_omnibor: None,
    };

    // Build the task matrix for building Artifacts (similar to GitHub CI)
    let mut tasks = vec![];
    
    // Get local targets that need to be built from the distribution graph
    let local_targets: std::collections::BTreeSet<&TripleNameRef> = dist.artifacts.iter()
        .filter(|artifact| !artifact.required_binaries.is_empty()) // Only artifacts that build binaries
        .flat_map(|artifact| &artifact.target_triples)
        .map(|triple| triple.as_ref())
        .collect();
    eprintln!("DEBUG: local_targets: {:?}", local_targets);
    
    eprintln!("DEBUG: Creating tasks for {} targets", local_targets.len());
    
    if !local_targets.is_empty() {
        use std::fmt::Write;
        
        // Use default ubuntu runner for all targets  
        let runner = dist_schema::GithubRunnerConfig {
            runner: GithubRunnerRef::from_str("ubuntu-22.04").to_owned(),
            host: TripleNameRef::from_str("x86_64-unknown-linux-gnu").to_owned(), 
            container: None,
        };
        let targets: Vec<&TripleNameRef> = local_targets.iter().copied().collect();
        
        eprintln!("DEBUG: Processing {} targets with runner", targets.len());
        let real_triple = runner.real_triple();
        eprintln!("DEBUG: real_triple: {:?}", real_triple);
        // Regenerate install strategy with repository URL for each job
        let job_dist_install_strategy = (DistInstallSettings {
            version: dist_version,
            url_override: dist.config.dist_url_override.as_deref(),
            repository_url: repository_url.clone(),
        })
        .install_strategy();
        let install_dist = job_dist_install_strategy.for_triple(&real_triple);
        eprintln!("DEBUG: install_dist created");
        
        let mut dist_args = String::from("--artifacts=local");
        for target in &targets {
            write!(dist_args, " --target={target}").unwrap();
        }
        eprintln!("DEBUG: dist_args: {}", dist_args);
        
        // For now, no system dependencies - this can be enhanced later
        let packages_install = None;
        
        eprintln!("DEBUG: About to push task for targets: {:?}", targets);
        tasks.push(dist_schema::GithubLocalJobConfig {
            targets: Some(targets.iter().copied().map(|s| s.to_owned()).collect()),
            cache_provider: None, // Use default caching for now
            runner,
            dist_args,
            install_dist: install_dist.to_owned(),
            install_cargo_auditable: None, // TODO: add support later
            install_omnibor: None, // TODO: add support later  
            packages_install,
        });
        eprintln!("DEBUG: Task pushed successfully");
    }
    
    let artifacts_matrix = dist_schema::GithubMatrix {
        include: tasks,
    };
    eprintln!("DEBUG: artifacts_matrix has {} items", artifacts_matrix.include.len());
    eprintln!("DEBUG: Continuing to create jobs...");
    
    // Create local artifacts jobs for template rendering
    let mut local_artifacts_jobs = Vec::new();
    eprintln!("DEBUG: About to create local artifacts jobs, local_targets.is_empty(): {}", local_targets.is_empty());
    if !local_targets.is_empty() {
        eprintln!("DEBUG: Creating local artifacts job");
        local_artifacts_jobs.push(ForgejoCiJob {
            id: "build-local-artifacts".to_string(),
            name: "build-local-artifacts".to_string(), 
            runner: "ubuntu-22.04".to_string(),
            only_if_release: None,
            permissions: None,
        });
    }
    eprintln!("DEBUG: local_artifacts_jobs has {} items", local_artifacts_jobs.len());

    // Build basic info structure
    Ok(ForgejoCiInfo {
        forgejo_ci_workflow_dir: workflow_dir,
        fail_fast,
        cache_builds,
        build_local_artifacts,
        dispatch_releases,
        release_branch,
        artifacts_matrix,
        pr_run_mode,
        global_task,
        plan_jobs: vec![],
        local_artifacts_jobs,
        global_artifacts_jobs: vec![],
        host_jobs: vec![],
        publish_jobs: vec![],
        user_publish_jobs: vec![],
        post_announce_jobs: vec![],
        hosting_providers,
        tag_namespace,
        root_permissions: None,
        create_release: true,
        // Use the same install strategy for coordinator
        dist_install_for_coordinator: dist_install_strategy.for_triple(&dist_schema::target_lexicon::Triple::host()),
        build_setup: config.build_setup.clone(),
    })
}