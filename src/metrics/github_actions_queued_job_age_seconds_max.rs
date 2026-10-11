use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct QueuedJobAgeSecondsMaxLabels {
    repository: String,
    runner_label: String,
}

pub type QueuedJobAgeSecondsMax = Family<QueuedJobAgeSecondsMaxLabels, Gauge>;
