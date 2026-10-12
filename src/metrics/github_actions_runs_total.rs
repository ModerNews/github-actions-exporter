use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::metric;
use crate::webhooks::envelope::WorkflowRunEnvelope;
use crate::webhooks::workflow_conclusion::WorkflowConclusion;

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

impl From<&WorkflowRunEnvelope> for RunsTotalLabels {
    fn from(value: &WorkflowRunEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value.workflow_run.name.clone(),
            event: value.workflow_run.event.clone(),
            conclusion: value
                .workflow_run
                .conclusion
                .clone()
                .unwrap_or(WorkflowConclusion::Unknown)
                .into(),
            attempt: if value.workflow_run.run_attempt > 1 {
                "retry".into()
            } else {
                "first".into()
            },
        }
    }
}
