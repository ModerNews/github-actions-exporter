use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobsInFlightLabels {
    repository: String,
    workflow_name: String,
    status: String,
    runner_label: String,
}

pub type JobsInFlight = Family<JobsInFlightLabels, Gauge>;
