use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobsTotalLabels {
    repository: String,
    workflow_name: String,
    job_name: String,
    conclusion: String,
    runner_group_name: String,
}

pub type JobsTotal = Family<JobsTotalLabels, Counter>;
