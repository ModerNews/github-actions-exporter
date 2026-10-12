use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ExporterDeliveriesTotalLabels {
    event: String,
    action: String,
    outcome: String,
}

metric! {
    ExporterDeliveriesTotal(Family<ExporterDeliveriesTotalLabels, Counter>),
    key: "github_actions_exporter_deliveries",
    help: "Webhook deliveries received, by what the exporter did with each",
    new: Family::default(),
}
