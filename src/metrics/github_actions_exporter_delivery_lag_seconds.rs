use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ExporterDeliveryLagSecondsLabels {
    event: String,
}

metric! {
    ExporterDeliveryLagSeconds(
        Family<ExporterDeliveryLagSecondsLabels, Histogram, fn() -> Histogram>
    ),
    key: "github_actions_exporter_delivery_lag_seconds",
    help: "Delay between the payload event timestamp and delivery receipt",
    new: Family::new_with_constructor(|| Histogram::new(BUCKETS)),
}

pub const BUCKETS: [f64; 7] = [1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0];
