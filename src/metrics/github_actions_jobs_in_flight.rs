use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

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
