use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::Ingester;
use crate::webhooks::referenced_workflow::ReferencedWorkflow;
use crate::webhooks::workflow_conclusion::WorkflowConclusion;
use crate::webhooks::workflow_generics::{Commit, PullRequest, Repository, User};
use crate::webhooks::workflow_status::WorkflowStatus;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRun {
    pub id: i64,
    pub node_id: String,
    pub run_number: i64,
    pub run_attempt: i64,
    pub workflow_id: i64,
    pub name: String,
    pub path: String,
    pub display_title: String,
    pub event: String,
    pub status: WorkflowStatus,
    // Null till completion
    pub conclusion: Option<WorkflowConclusion>,
    pub created_at: DateTime<Utc>,
    pub run_started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub head_branch: String,
    pub head_sha: String,
    pub head_commit: Commit,
    pub repository: Repository,
    pub head_repository: Repository,
    pub actor: User,
    pub triggering_actor: User,
    pub pull_requests: Vec<PullRequest>,
    #[serde(default)]
    pub referenced_workflows: Vec<ReferencedWorkflow>,
    pub check_suite_id: i64,
    pub check_suite_node_id: String,
    pub url: String,
    pub html_url: String,
    pub jobs_url: String,
    pub logs_url: String,
    pub check_suite_url: String,
    pub artifacts_url: String,
    pub cancel_url: String,
    pub rerun_url: String,
    pub workflow_url: String,
    /// Only populated once `run_attempt > 1`.
    pub previous_attempt_url: Option<String>,
}

impl WorkflowRun {
    pub fn queue_delay(&self) -> chrono::TimeDelta {
        self.run_started_at - self.created_at
    }
}

impl Ingester for WorkflowRun {
    fn ingest(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(?self);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const REAL_RUN: &str = include_str!("../../tests/fixtures/workflow_run.json");

    fn real_run() -> WorkflowRun {
        serde_json::from_str(REAL_RUN).expect("real GitHub payload must deserialize")
    }

    #[test]
    fn deserializes_a_real_github_payload() {
        let run = real_run();
        assert_eq!(run.id, 37610094675);
        assert_eq!(run.name, "Secret Scanning");
        assert_eq!(run.event, "pull_request");
        assert_eq!(run.status, WorkflowStatus::Completed);
        assert_eq!(run.conclusion, Some(WorkflowConclusion::Success));
    }

    #[test]
    fn every_url_field_is_populated() {
        let run = real_run();
        for (name, value) in [
            ("url", &run.url),
            ("html_url", &run.html_url),
            ("jobs_url", &run.jobs_url),
            ("logs_url", &run.logs_url),
            ("check_suite_url", &run.check_suite_url),
            ("artifacts_url", &run.artifacts_url),
            ("cancel_url", &run.cancel_url),
            ("rerun_url", &run.rerun_url),
            ("workflow_url", &run.workflow_url),
        ] {
            assert!(value.starts_with("https://"), "{name} was {value:?}");
        }
        assert_eq!(run.run_attempt, 1);
        assert_eq!(run.previous_attempt_url, None);
    }

    #[test]
    fn referenced_workflow_attributes_the_ci_lib_version() {
        let run = real_run();
        let referenced = run
            .referenced_workflows
            .first()
            .expect("this run calls a reusable workflow");
        assert_eq!(
            referenced.path,
            "WMS-DEV/infra-ci-lib/.github/workflows/secret-scanning.yml@v1.1.2"
        );
        assert_eq!(referenced.r#ref.as_deref(), Some("refs/tags/v1.1.2"));
        assert_eq!(referenced.sha.len(), 40);
    }

    #[test]
    fn referenced_workflows_defaults_when_absent() {
        let mut json: serde_json::Value = serde_json::from_str(REAL_RUN).unwrap();
        json.as_object_mut().unwrap().remove("referenced_workflows");
        let run: WorkflowRun = serde_json::from_value(json).expect("must not require the field");
        assert!(run.referenced_workflows.is_empty());
    }

    #[test]
    fn parses_rfc3339_timestamps() {
        let run = real_run();
        assert_eq!(run.created_at.to_rfc3339(), "2026-10-07T10:50:56+00:00");
        assert!(run.updated_at > run.created_at);
    }

    #[test]
    fn run_level_queue_delay_is_not_the_runner_wait() {
        let run = real_run();
        assert_eq!(run.queue_delay().num_seconds(), 0);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let mut json: serde_json::Value = serde_json::from_str(REAL_RUN).unwrap();
        json.as_object_mut()
            .unwrap()
            .insert("some_future_field".into(), serde_json::json!({"a": 1}));
        assert!(serde_json::from_value::<WorkflowRun>(json).is_ok());
    }

    #[test]
    fn id_over_i32() {
        #[derive(serde::Deserialize)]
        struct Narrow {
            #[allow(dead_code)]
            id: u32,
        }
        let json = serde_json::json!({"id": 37610094675_i64});
        assert!(serde_json::from_value::<Narrow>(json).is_err());
        assert!(37610094675_i64 > u32::MAX as i64);
    }
}
