use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobQueueSecondsLabels {
    repository: String,
    runner_group_name: String,
    runner_label: String,
}

pub type JobQueueSeconds = Family<JobQueueSecondsLabels, Histogram, fn() -> Histogram>;

pub const BUCKETS: [f64; 11] = [
    1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
];
