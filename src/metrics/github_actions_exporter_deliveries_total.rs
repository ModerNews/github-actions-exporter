use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ExporterDeliveriesTotalLabels {
    event: String,
    action: String,
    outcome: String,
}

pub type ExporterDeliveriesTotal = Family<ExporterDeliveriesTotalLabels, Counter>;
