use prometheus_client::metrics::gauge::Gauge;

use crate::metric;

metric! {
    ExporterLastDeliveryTimestampSeconds(Gauge),
    key: "github_actions_exporter_last_delivery_timestamp_seconds",
    help: "Unix time of the latest accepted delivery",
    new: Gauge::default(),
}
