//! forgejo ci config

use dist_schema::{
    GithubRunner, GithubRunnerConfig, GithubRunnerConfigInput, StringLikeOr,
    TripleName,
};


use super::*;

/// forgejo ci config (raw from file)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ForgejoCiLayer {
    /// Common options
    #[serde(flatten)]
    pub common: CommonCiLayer,

    /// Custom Forgejo runners, mapped by triple target
    /// (Forgejo uses same runner syntax as GitHub Actions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runners: Option<SortedMap<TripleName, StringLikeOr<GithubRunner, GithubRunnerConfigInput>>>,

    /// Custom permissions for jobs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<SortedMap<String, GithubPermissionMap>>,

    /// Custom build setup script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_setup: Option<String>,

    /// Use these commits for actions (if using GitHub-compatible actions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_commits: Option<SortedMap<String, String>>,

    /// Custom Forgejo runner for plan job
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_runner: Option<String>,

    /// Custom Forgejo runner for build jobs  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_runner: Option<String>,

}

/// forgejo ci config (final)
#[derive(Debug, Default, Clone)]
pub struct ForgejoCiConfig {
    /// Common options
    pub common: CommonCiConfig,

    /// Custom Forgejo runners, mapped by triple target
    pub runners: SortedMap<TripleName, GithubRunnerConfig>,

    /// Custom permissions for jobs
    pub permissions: SortedMap<String, GithubPermissionMap>,

    /// Custom build setup script
    pub build_setup: Option<String>,

    /// Use these commits for actions (if using GitHub-compatible actions)
    pub action_commits: SortedMap<String, String>,

    /// Custom Forgejo runner for plan job
    pub plan_runner: Option<String>,

    /// Custom Forgejo runner for build jobs
    pub build_runner: Option<String>,

}

impl ForgejoCiConfig {
    /// Get default configuration for workspace
    pub fn defaults_for_workspace(
        _workspaces: &WorkspaceGraph,
        common: &CommonCiConfig,
    ) -> ForgejoCiConfig {
        Self {
            common: common.clone(),
            runners: SortedMap::new(),
            permissions: SortedMap::new(),
            build_setup: None,
            action_commits: SortedMap::new(),
            plan_runner: None,
            build_runner: None,
        }
    }
}

impl ApplyLayer for ForgejoCiConfig {
    type Layer = ForgejoCiLayer;
    fn apply_layer(
        &mut self,
        ForgejoCiLayer {
            common,
            runners: _,
            permissions,
            build_setup,
            action_commits,
            plan_runner,
            build_runner,
        }: Self::Layer,
    ) {
        self.common.apply_layer(common);
        // TODO: Implement runner configuration properly
        if let Some(perms) = permissions {
            self.permissions.extend(perms);
        }
        if let Some(setup) = build_setup {
            self.build_setup = Some(setup);
        }
        if let Some(commits) = action_commits {
            self.action_commits.extend(commits);
        }
        if plan_runner.is_some() {
            self.plan_runner = plan_runner;
        }
        if build_runner.is_some() {
            self.build_runner = build_runner;
        }
    }
}

impl ApplyLayer for ForgejoCiLayer {
    type Layer = ForgejoCiLayer;
    fn apply_layer(
        &mut self,
        ForgejoCiLayer {
            common,
            runners: _,
            permissions,
            build_setup,
            action_commits,
            plan_runner,
            build_runner,
        }: Self::Layer,
    ) {
        self.common.apply_layer(common);
        // TODO: Implement runner configuration properly  
        if let Some(perms) = permissions {
            if self.permissions.is_none() {
                self.permissions = Some(SortedMap::new());
            }
            self.permissions.as_mut().unwrap().extend(perms);
        }
        if build_setup.is_some() {
            self.build_setup = build_setup;
        }
        if let Some(commits) = action_commits {
            if self.action_commits.is_none() {
                self.action_commits = Some(SortedMap::new());
            }
            self.action_commits.as_mut().unwrap().extend(commits);
        }
        if plan_runner.is_some() {
            self.plan_runner = plan_runner;
        }
        if build_runner.is_some() {
            self.build_runner = build_runner;
        }
        // TODO: Handle runners properly when we implement that
    }
}