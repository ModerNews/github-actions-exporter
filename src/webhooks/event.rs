//! Event identifiers.
use crate::{FromHeader, ParseHeader};

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

impl FromHeader for WebhookEvent {
    const HEADER: &'static str = "x-github-event";
}

impl ParseHeader for WebhookEvent {
    fn parse_header(value: &str) -> Option<Self> {
        Some(match value {
            "workflow_run" => Self::WorkflowRun,
            "workflow_job" => Self::WorkflowJob,
            "ping" => Self::Ping,
            s => Self::Other(s.into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_events_parse_and_fall_back() {
        assert_eq!(
            WebhookEvent::parse_header("workflow_job"),
            Some(WebhookEvent::WorkflowJob)
        );
        assert_eq!(WebhookEvent::parse_header("ping"), Some(WebhookEvent::Ping));
        assert_eq!(
            WebhookEvent::parse_header("issues"),
            Some(WebhookEvent::Other("issues".into()))
        );
    }
}
