use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RunsInFlightLabels {
    repository: String,
    workflow_name: String,
    event: String,
}

pub type RunsInFlight = Family<RunsInFlightLabels, Gauge>;
