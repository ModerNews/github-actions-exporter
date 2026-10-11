use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ReferencedWorkflowRunsTotalLabels {
    repository: String,
    referenced_workflow: String,
    version: String,
}

pub type ReferencedWorkflowRunsTotal = Family<ReferencedWorkflowRunsTotalLabels, Counter>;
