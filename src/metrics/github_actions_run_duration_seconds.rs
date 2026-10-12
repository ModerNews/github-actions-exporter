use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

use crate::metric;
use crate::webhooks::envelope::WorkflowRunEnvelope;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RunDurationSecondsLabels {
    repository: String,
    workflow_name: String,
    event: String,
}

metric! {
    RunDurationSeconds(Family<RunDurationSecondsLabels, Histogram, fn() -> Histogram>),
    key: "github_actions_run_duration_seconds",
    help: "Workflow run wall-clock time, with updated_at as a proxy for completion",
    new: Family::new_with_constructor(|| Histogram::new(BUCKETS)),
}

pub const BUCKETS: [f64; 10] = [
    30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 21600.0,
];

impl From<&WorkflowRunEnvelope> for RunDurationSecondsLabels {
    fn from(value: &WorkflowRunEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value.workflow_run.name.clone(),
            event: value.workflow_run.event.clone(),
        }
    }
}
