use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::metric;
use crate::webhooks::envelope::WorkflowRunEnvelope;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RunsInFlightLabels {
    repository: String,
    workflow_name: String,
    event: String,
}

metric! {
    RunsInFlight(Family<RunsInFlightLabels, Gauge>),
    key: "github_actions_runs_in_flight",
    help: "Workflow runs below terminal rank in observed state",
    new: Family::default(),
}

impl From<&WorkflowRunEnvelope> for RunsInFlightLabels {
    fn from(value: &WorkflowRunEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value.workflow_run.name.clone(),
            event: value.workflow_run.event.clone(),
        }
    }
}
