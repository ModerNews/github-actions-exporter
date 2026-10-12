use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ReferencedWorkflowRunsTotalLabels {
    repository: String,
    referenced_workflow: String,
    version: String,
}

metric! {
    ReferencedWorkflowRunsTotal(Family<ReferencedWorkflowRunsTotalLabels, Counter>),
    key: "github_actions_referenced_workflow_runs",
    help: "Reusable workflow invocations by terminal runs",
    new: Family::default(),
}
