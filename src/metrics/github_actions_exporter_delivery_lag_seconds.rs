use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, histogram::Histogram};

use crate::metric;
use crate::webhooks::envelope::{WorkflowJobEnvelope, WorkflowRunEnvelope};
use crate::webhooks::event::WebhookEvent;

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

impl From<&WorkflowRunEnvelope> for ExporterDeliveryLagSecondsLabels {
    fn from(_: &WorkflowRunEnvelope) -> Self {
        Self {
            event: WebhookEvent::WorkflowRun.as_str().into(),
        }
    }
}

impl From<&WorkflowJobEnvelope> for ExporterDeliveryLagSecondsLabels {
    fn from(_: &WorkflowJobEnvelope) -> Self {
        Self {
            event: WebhookEvent::WorkflowJob.as_str().into(),
        }
    }
}
