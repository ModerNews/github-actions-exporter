use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{family::Family, gauge::Gauge};

use crate::entry_map::RunEntry;
use crate::metric;

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

impl From<&RunEntry> for RunsInFlightLabels {
    fn from(value: &RunEntry) -> Self {
        Self {
            repository: value.repository.clone(),
            workflow_name: value.workflow_name.clone(),
            event: value.event.clone(),
        }
    }
}
