//! Known runner names and their associated targets

use dist_schema::{target_lexicon::Triple, ForgejoRunnerRef, TripleNameRef};
use crate::platform::targets as t;
use std::collections::HashMap;
use tracing::warn;

static KNOWN_FORGEJO_RUNNERS: std::sync::LazyLock<HashMap<&'static str, &'static TripleNameRef>> = std::sync::LazyLock::new(|| {
    let mut runners = HashMap::new();
    
    // Common Forgejo/Gitea Actions runners
    runners.insert("ubuntu-latest", t::TARGET_X64_LINUX_GNU);
    runners.insert("ubuntu-22.04", t::TARGET_X64_LINUX_GNU);
    runners.insert("ubuntu-20.04", t::TARGET_X64_LINUX_GNU);
    runners.insert("docker", t::TARGET_X64_LINUX_GNU);
    
    runners
});

/// Get the target triple for a known forgejo runner, if we recognize it
pub fn target_for_forgejo_runner(runner: &ForgejoRunnerRef) -> Option<&'static TripleNameRef> {
    KNOWN_FORGEJO_RUNNERS.get(runner.as_str()).copied()
}

/// Get the target triple for a forgejo runner, defaulting to linux if unknown
pub fn target_for_forgejo_runner_or_default(runner: &ForgejoRunnerRef) -> &'static TripleNameRef {
    const DEFAULT_ASSUMED_TARGET: &TripleNameRef = t::TARGET_X64_LINUX_GNU;

    target_for_forgejo_runner(runner).unwrap_or_else(|| {
        warn!(
            "don't know the triple for forgejo runner '{}', assuming {}",
            runner, DEFAULT_ASSUMED_TARGET
        );
        DEFAULT_ASSUMED_TARGET
    })
}

/// Get the parsed target triple for a forgejo runner, defaulting to linux if unknown
pub fn triple_for_forgejo_runner_or_default(runner: &ForgejoRunnerRef) -> Triple {
    target_for_forgejo_runner_or_default(runner).parse().unwrap()
}