use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::entry_map::JobEntry;
use crate::metric;

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

impl From<&JobEntry> for JobsInFlightLabels {
    fn from(value: &JobEntry) -> Self {
        Self {
            repository: value.repository.clone(),
            workflow_name: value.workflow_name.clone(),
            status: value.status.clone().into(),
            runner_label: value.runner_label.clone(),
        }
    }
}
