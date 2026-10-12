use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::metric;
use crate::webhooks::envelope::WorkflowRunEnvelope;
use crate::webhooks::referenced_workflow::ReferencedWorkflow;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ReferencedWorkflowRunsTotalLabels {
    repository: String,
    referenced_workflow: String,
    version: String,
}

metric! {
    ReferencedWorkflowRunsTotal(Family<ReferencedWorkflowRunsTotalLabels, Counter>),
    key: "github_actions_referenced_workflow_runs",
    help: "Reusable workflow invocations by terminal runs",
    new: Family::default(),
}

impl From<(&WorkflowRunEnvelope, &ReferencedWorkflow)> for ReferencedWorkflowRunsTotalLabels {
    fn from((envelope, referenced): (&WorkflowRunEnvelope, &ReferencedWorkflow)) -> Self {
        let (path, pinned) = referenced
            .path
            .rsplit_once('@')
            .map_or((referenced.path.as_str(), None), |(path, pin)| {
                (path, Some(pin))
            });

        Self {
            repository: envelope.repository.full_name.clone(),
            referenced_workflow: path.into(),
            version: match referenced.r#ref {
                Some(_) => pinned.unwrap_or("unknown").into(),
                None => "sha".into(),
            },
        }
    }
}
