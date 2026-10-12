use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::metric;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RunsTotalLabels {
    repository: String,
    workflow_name: String,
    event: String,
    conclusion: String,
    attempt: String,
}

metric! {
    RunsTotal(Family<RunsTotalLabels, Counter>),
    key: "github_actions_runs",
    help: "Workflow runs that reached a terminal state",
    new: Family::default(),
}
