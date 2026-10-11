use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ExporterDeliveryLagSecondsLabels {
    event: String,
}

pub type ExporterDeliveryLagSeconds =
    Family<ExporterDeliveryLagSecondsLabels, Histogram, fn() -> Histogram>;

pub const BUCKETS: [f64; 7] = [1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0];
