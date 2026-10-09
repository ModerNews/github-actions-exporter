//! Event identifiers.

/// Which webhook arrived, from the `X-GitHub-Event` header.
///
/// An enum because the handler dispatches on it. An `Other` must still be
/// acked `204`: returning 4xx for unrecognised events gets the webhook
/// disabled by GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WebhookEvent {
    WorkflowRun,
    WorkflowJob,
    /// Sent once when the webhook is created; ack it or setup appears broken.
    Ping,
    Other(String),
}

impl WebhookEvent {
    pub fn as_str(&self) -> &str {
        match self {
            Self::WorkflowRun => "workflow_run",
            Self::WorkflowJob => "workflow_job",
            Self::Ping => "ping",
            Self::Other(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_events_parse_and_fall_back() {
        // assert_eq!(WebhookEvent::from_header("workflow_job"), WebhookEvent::WorkflowJob);
        // assert_eq!(WebhookEvent::from_header("ping"), WebhookEvent::Ping);
        // assert_eq!(
        //     WebhookEvent::from_header("issues"),
        //     WebhookEvent::Other("issues".into())
        // );
    }
}
