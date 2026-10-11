use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RunsTotalLabels {
    repository: String,
    workflow_name: String,
    event: String,
    conclusion: String,
    attempt: String,
}

pub type RunsTotal = Family<RunsTotalLabels, Counter>;
