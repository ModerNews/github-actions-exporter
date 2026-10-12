use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::metric;
use crate::webhooks::envelope::WorkflowJobEnvelope;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct QueuedJobAgeSecondsMaxLabels {
    repository: String,
    runner_label: String,
}

metric! {
    QueuedJobAgeSecondsMax(Family<QueuedJobAgeSecondsMaxLabels, Gauge>),
    key: "github_actions_queued_job_age_seconds_max",
    help: "Age of the oldest job still queued or waiting",
    new: Family::default(),
}

impl From<&WorkflowJobEnvelope> for QueuedJobAgeSecondsMaxLabels {
    fn from(value: &WorkflowJobEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            runner_label: value.workflow_job.runner_label(),
        }
    }
}
