pub mod envelope;
pub mod event;
pub mod referenced_workflow;
pub mod workflow_conclusion;
pub mod workflow_generics;
pub mod workflow_job;
pub mod workflow_run;
pub mod workflow_status;

use std::error::Error;

use crate::metrics::{Ingester, Metrics};
use crate::webhooks::envelope::{WorkflowJobEnvelope, WorkflowRunEnvelope};
use crate::webhooks::event::WebhookEvent;

/// Deserializes a delivery body against the envelope its event names, and
/// hands it to that envelope's [`Ingester`]. Events we do not model are a
/// no-op, not an error.
pub async fn process_webhook(
    event: WebhookEvent,
    body: &[u8],
    metrics: &Metrics,
) -> Result<(), Box<dyn Error>> {
    match event {
        WebhookEvent::WorkflowRun => {
            serde_json::from_slice::<WorkflowRunEnvelope>(body)?.ingest(metrics)
        }
        WebhookEvent::WorkflowJob => {
            serde_json::from_slice::<WorkflowJobEnvelope>(body)?.ingest(metrics)
        }
        _ => Ok(()),
    }
}
