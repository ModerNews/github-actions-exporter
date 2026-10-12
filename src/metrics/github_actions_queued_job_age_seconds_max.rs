use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct QueuedJobAgeSecondsMaxLabels {
    repository: String,
    runner_label: String,
}

metric! {
    QueuedJobAgeSecondsMax(Family<QueuedJobAgeSecondsMaxLabels, Gauge>),
    key: "github_actions_queued_job_age_seconds_max",
    help: "Age of the oldest job still queued or waiting",
    new: Family::default(),
}
