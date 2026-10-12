use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

use crate::metric;
use crate::webhooks::envelope::WorkflowJobEnvelope;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobQueueSecondsLabels {
    repository: String,
    runner_group_name: String,
    runner_label: String,
}

metric! {
    JobQueueSeconds(Family<JobQueueSecondsLabels, Histogram, fn() -> Histogram>),
    key: "github_actions_job_queue_seconds",
    help: "Time a job spent queued before pickup",
    new: Family::new_with_constructor(|| Histogram::new(BUCKETS)),
}

pub const BUCKETS: [f64; 11] = [
    1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
];

impl From<&WorkflowJobEnvelope> for JobQueueSecondsLabels {
    fn from(value: &WorkflowJobEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            runner_group_name: value
                .workflow_job
                .runner_group_name
                .clone()
                .unwrap_or("".into()),
            runner_label: value.workflow_job.runner_label(),
        }
    }
}
