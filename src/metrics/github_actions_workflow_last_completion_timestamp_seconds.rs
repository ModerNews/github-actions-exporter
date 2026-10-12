use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct WorkflowLastCompletionTimestampSecondsLabels {
    repository: String,
    workflow_name: String,
    conclusion: String,
}

metric! {
    WorkflowLastCompletionTimestampSeconds(
        Family<WorkflowLastCompletionTimestampSecondsLabels, Gauge>
    ),
    key: "github_actions_workflow_last_completion_timestamp_seconds",
    help: "Unix time of the last terminal run per workflow and conclusion",
    new: Family::default(),
}
