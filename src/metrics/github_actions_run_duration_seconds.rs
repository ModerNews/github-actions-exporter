use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RunDurationSecondsLabels {
    repository: String,
    workflow_name: String,
    event: String,
}

pub type RunDurationSeconds = Family<RunDurationSecondsLabels, Histogram, fn() -> Histogram>;

pub const BUCKETS: [f64; 10] = [
    30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 21600.0,
];
