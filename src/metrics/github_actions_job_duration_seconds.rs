use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

use crate::metric;
use crate::webhooks::envelope::WorkflowJobEnvelope;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobDurationSecondsLabels {
    repository: String,
    workflow_name: String,
    job_name: String,
    runner_group_name: String,
}

metric! {
    JobDurationSeconds(Family<JobDurationSecondsLabels, Histogram, fn() -> Histogram>),
    key: "github_actions_job_duration_seconds",
    help: "Job execution time, from pickup to terminal",
    new: Family::new_with_constructor(|| Histogram::new(BUCKETS)),
}

pub const BUCKETS: [f64; 11] = [
    5.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1200.0, 1800.0, 3600.0, 7200.0,
];

impl From<&WorkflowJobEnvelope> for JobDurationSecondsLabels {
    fn from(value: &WorkflowJobEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value
                .workflow_job
                .workflow_name
                .clone()
                .unwrap_or("".into()),
            job_name: value.workflow_job.name.clone(),
            runner_group_name: value
                .workflow_job
                .runner_group_name
                .clone()
                .unwrap_or("".into()),
        }
    }
}
