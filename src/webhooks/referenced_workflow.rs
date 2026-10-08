use serde::Deserialize;

/// A reusable workflow a run called via `workflow_call`.
///
/// This is how a run is attributed to `infra-ci-lib`: the called workflow emits
/// no `workflow_run` of its own, so this array is the only link.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReferencedWorkflow {
    /// `owner/repo/.github/workflows/name.yml@ref` — the pinned version is
    /// embedded here, e.g. `…/secret-scanning.yml@v1.1.2`.
    pub path: String,
    /// Commit the ref resolved to.
    pub sha: String,
    /// `refs/tags/v1.1.2`. Optional: absent when pinned to a raw SHA.
    pub r#ref: Option<String>,
}
