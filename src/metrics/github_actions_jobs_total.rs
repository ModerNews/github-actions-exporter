use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family};

use crate::webhooks::envelope::WorkflowJobEnvelope;
use crate::webhooks::workflow_conclusion::WorkflowConclusion;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct JobsTotalLabels {
    repository: String,
    workflow_name: String,
    job_name: String,
    conclusion: String,
    runner_group_name: String,
}

pub type JobsTotal = Family<JobsTotalLabels, Counter>;

impl From<&WorkflowJobEnvelope> for JobsTotalLabels {
    fn from(value: &WorkflowJobEnvelope) -> Self {
        Self {
            repository: value.repository.full_name.clone(),
            workflow_name: value
                .workflow_job
                .workflow_name
                .clone()
                .unwrap_or("".into()),
            job_name: value.workflow_job.name.clone(),
            conclusion: value
                .workflow_job
                .conclusion
                .clone()
                .unwrap_or(WorkflowConclusion::Unknown)
                .into(),
            runner_group_name: value
                .workflow_job
                .runner_group_name
                .clone()
                .unwrap_or("".into()),
        }
    }
}
