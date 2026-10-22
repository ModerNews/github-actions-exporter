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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Processed,
    Ignored,
    RejectedSignature,
    RejectedHeader,
    ParseError,
}

impl DeliveryOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Processed => "processed",
            Self::Ignored => "ignored",
            Self::RejectedSignature => "rejected_signature",
            Self::RejectedHeader => "rejected_header",
            Self::ParseError => "parse_error",
        }
    }
}

impl ExporterDeliveriesTotalLabels {
    pub fn new(event: &str, action: &str, outcome: DeliveryOutcome) -> Self {
        Self {
            event: event.to_owned(),
            action: action.to_owned(),
            outcome: outcome.as_str().to_owned(),
        }
    }
}
