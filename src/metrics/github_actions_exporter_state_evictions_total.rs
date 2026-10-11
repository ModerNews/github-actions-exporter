use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ExporterStateEvictionsTotalLabels {
    entity: String,
    reason: String,
}

pub type ExporterStateEvictionsTotal = Family<ExporterStateEvictionsTotalLabels, Counter>;
