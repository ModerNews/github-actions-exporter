use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::metric;
use crate::webhooks::envelope::WorkflowRunEnvelope;
use crate::webhooks::workflow_conclusion::WorkflowConclusion;

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

impl From<&WorkflowRunEnvelope> for WorkflowLastCompletionTimestampSecondsLabels {
    fn from(value: &WorkflowRunEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value.workflow_run.name.clone(),
            conclusion: value
                .workflow_run
                .conclusion
                .clone()
                .unwrap_or(WorkflowConclusion::Unknown)
                .into(),
        }
    }
}
