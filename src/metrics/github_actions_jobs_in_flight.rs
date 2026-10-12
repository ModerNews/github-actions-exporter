use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::metric;
use crate::webhooks::envelope::WorkflowJobEnvelope;
use crate::webhooks::workflow_status::WorkflowStatus;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobsInFlightLabels {
    repository: String,
    workflow_name: String,
    status: String,
    runner_label: String,
}

metric! {
    JobsInFlight(Family<JobsInFlightLabels, Gauge>),
    key: "github_actions_jobs_in_flight",
    help: "Jobs below terminal rank in observed state",
    new: Family::default(),
}

impl From<&WorkflowJobEnvelope> for JobsInFlightLabels {
    fn from(value: &WorkflowJobEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value
                .workflow_job
                .workflow_name
                .clone()
                .unwrap_or("".into()),
            status: value.workflow_job.status.clone().into(),
            runner_label: value.workflow_job.runner_label(),
        }
    }
}

impl JobsInFlightLabels {
    pub fn with_status(mut self, status: WorkflowStatus) -> Self {
        self.status = status.into();
        self
    }
}
