use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ExporterStateEvictionsTotalLabels {
    entity: String,
    reason: String,
}

metric! {
    ExporterStateEvictionsTotal(Family<ExporterStateEvictionsTotalLabels, Counter>),
    key: "github_actions_exporter_state_evictions",
    help: "Entity state entries evicted past the max lifetime; alert on reason=lost only",
    new: Family::default(),
}

impl ExporterStateEvictionsTotalLabels {
    pub fn new(entity: &str, reason: &str) -> Self {
        Self {
            entity: entity.to_owned(),
            reason: reason.to_owned(),
        }
    }
}
