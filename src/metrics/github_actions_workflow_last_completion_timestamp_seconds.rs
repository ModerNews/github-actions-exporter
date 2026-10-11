use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct WorkflowLastCompletionTimestampSecondsLabels {
    repository: String,
    workflow_name: String,
    conclusion: String,
}

pub type WorkflowLastCompletionTimestampSeconds =
    Family<WorkflowLastCompletionTimestampSecondsLabels, Gauge>;
