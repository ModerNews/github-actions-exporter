//! Delivery envelopes.
//!
//! GitHub does not POST a `workflow_run` / `workflow_job` object directly — it
//! nests the payload under a key named after the event and wraps it with
//! `action` plus the delivery context. Parsing a payload struct against the
//! raw body fails with `missing field `id``, so every event gets an envelope.

use serde::Deserialize;

use crate::Ingester;
use crate::webhooks::workflow_generics::{Installation, Organization, Repository, User, Workflow};
use crate::webhooks::workflow_job::WorkflowJob;
use crate::webhooks::workflow_run::WorkflowRun;

/// Snake_case on the wire, same as [`WorkflowStatus`]; the run-level actions
/// are a subset of the job-level ones — there is no `queued` for a run.
///
/// [`WorkflowStatus`]: crate::webhooks::workflow_status::WorkflowStatus
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunAction {
    Requested,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowJobAction {
    Queued,
    InProgress,
    Completed,
    Waiting,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRunEnvelope {
    pub action: WorkflowRunAction,
    pub workflow_run: WorkflowRun,
    /// The definition the run came from. Nullable on the wire when the `.yml`
    /// has been deleted since the run started, so a tightened type here would
    /// drop exactly the deliveries you most want to see.
    #[serde(default)]
    pub workflow: Option<Workflow>,
    pub repository: Repository,
    pub sender: User,
    /// Absent on personal-account repositories.
    #[serde(default)]
    pub organization: Option<Organization>,
    /// Only sent when the webhook is delivered via a GitHub App.
    #[serde(default)]
    pub installation: Option<Installation>,
}

/// The job payload carries no repository of its own, so the envelope's
/// `repository` is the only place the job's repo is named.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowJobEnvelope {
    pub action: WorkflowJobAction,
    pub workflow_job: WorkflowJob,
    pub repository: Repository,
    pub sender: User,
    #[serde(default)]
    pub organization: Option<Organization>,
    #[serde(default)]
    pub installation: Option<Installation>,
}

impl Ingester for WorkflowRunEnvelope {
    fn ingest(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(
            action = ?self.action,
            repo = %self.repository.full_name,
            "workflow_run"
        );
        self.workflow_run.ingest()
    }
}

impl Ingester for WorkflowJobEnvelope {
    fn ingest(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(
            action = ?self.action,
            repo = %self.repository.full_name,
            "workflow_job"
        );
        self.workflow_job.ingest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhooks::workflow_generics::WorkflowState;
    use serde_json::{Value, json};

    const REAL_RUN: &str = include_str!("../../tests/fixtures/workflow_run.json");

    /// Rebuilds the body GitHub actually sends from the inner-object fixture:
    /// the run's own `repository` and `actor` double as the envelope's
    /// `repository` and `sender`, so this stays a real payload throughout.
    fn run_envelope_json() -> Value {
        let inner: Value = serde_json::from_str(REAL_RUN).unwrap();
        json!({
            "action": "completed",
            "workflow_run": inner.clone(),
            "workflow": workflow_json(),
            "repository": inner["repository"].clone(),
            "sender": inner["actor"].clone(),
        })
    }

    /// Shaped from GitHub's documented `workflow` object — note the UTC-offset
    /// timestamps, which this object uses instead of the `Z` form the run
    /// payload uses.
    fn workflow_json() -> Value {
        json!({
            "id": 161335,
            "node_id": "MDg6V29ya2Zsb3cxNjEzMzU=",
            "name": "Secret Scanning",
            "path": ".github/workflows/secret-scanning.yml",
            "state": "active",
            "created_at": "2020-01-08T23:48:37.000-08:00",
            "updated_at": "2020-01-08T23:50:21.000-08:00",
            "url": "https://api.github.com/repos/WMS-DEV/infra-charts/actions/workflows/161335",
            "html_url": "https://github.com/WMS-DEV/infra-charts/blob/main/.github/workflows/secret-scanning.yml",
            "badge_url": "https://github.com/WMS-DEV/infra-charts/workflows/Secret%20Scanning/badge.svg"
        })
    }

    #[test]
    fn envelope_parses_and_exposes_the_inner_run() {
        let env: WorkflowRunEnvelope =
            serde_json::from_value(run_envelope_json()).expect("real delivery body must parse");
        assert_eq!(env.action, WorkflowRunAction::Completed);
        assert_eq!(env.workflow_run.id, 37610094675);
        assert_eq!(env.workflow_run.name, "Secret Scanning");
        assert_eq!(env.repository.full_name, "WMS-DEV/infra-charts");
        assert_eq!(env.sender.login, env.workflow_run.actor.login);
    }

    /// The exact bug the envelope exists to fix: the delivery body parsed as
    /// the inner payload fails on the payload's first required field.
    #[test]
    fn inner_payload_cannot_parse_the_delivery_body() {
        let err = serde_json::from_value::<WorkflowRun>(run_envelope_json())
            .expect_err("the envelope is not a WorkflowRun");
        assert!(
            err.to_string().contains("missing field `id`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn the_workflow_definition_is_deserialized_not_dropped() {
        let env: WorkflowRunEnvelope = serde_json::from_value(run_envelope_json()).unwrap();
        let wf = env.workflow.expect("the envelope carries the definition");
        assert_eq!(wf.id, 161335);
        assert_eq!(wf.path, ".github/workflows/secret-scanning.yml");
        assert_eq!(wf.state, WorkflowState::Active);
        // offset form normalised to UTC
        assert_eq!(wf.created_at.to_rfc3339(), "2020-01-09T07:48:37+00:00");
    }

    /// Nullable rather than required: a run whose `.yml` was deleted still
    /// reports, and that is the delivery worth keeping.
    #[test]
    fn a_missing_workflow_definition_does_not_fail_the_delivery() {
        let mut v = run_envelope_json();
        v.as_object_mut().unwrap().remove("workflow");
        assert!(
            serde_json::from_value::<WorkflowRunEnvelope>(v)
                .expect("absent workflow must parse")
                .workflow
                .is_none()
        );

        let mut v = run_envelope_json();
        v.as_object_mut()
            .unwrap()
            .insert("workflow".into(), Value::Null);
        assert!(
            serde_json::from_value::<WorkflowRunEnvelope>(v)
                .expect("null workflow must parse")
                .workflow
                .is_none()
        );
    }

    /// A field renamed or retyped inside `workflow` must fail loudly — the
    /// whole point of typing it instead of letting serde skip it.
    #[test]
    fn a_malformed_workflow_definition_is_not_silently_skipped() {
        let mut v = run_envelope_json();
        let mut wf = workflow_json();
        wf.as_object_mut()
            .unwrap()
            .insert("state".into(), json!("paused"));
        v.as_object_mut().unwrap().insert("workflow".into(), wf);
        assert!(serde_json::from_value::<WorkflowRunEnvelope>(v).is_err());
    }

    #[test]
    fn org_and_installation_are_optional() {
        let env: WorkflowRunEnvelope = serde_json::from_value(run_envelope_json()).unwrap();
        assert!(env.organization.is_none());
        assert!(env.installation.is_none());

        let mut v = run_envelope_json();
        let obj = v.as_object_mut().unwrap();
        obj.insert("installation".into(), json!({"id": 9, "node_id": "MDIzOg"}));
        let env: WorkflowRunEnvelope = serde_json::from_value(v).unwrap();
        assert_eq!(env.installation.map(|i| i.id), Some(9));
    }

    #[test]
    fn unknown_envelope_fields_are_ignored() {
        let mut v = run_envelope_json();
        v.as_object_mut()
            .unwrap()
            .insert("some_future_field".into(), json!({"a": 1}));
        assert!(serde_json::from_value::<WorkflowRunEnvelope>(v).is_ok());
    }

    /// The job half of the fix: `repository` lives only on the envelope here,
    /// so a job delivery is unusable without it.
    #[test]
    fn job_envelope_parses_and_carries_the_repository() {
        let inner: Value = serde_json::from_str(REAL_RUN).unwrap();
        let body = json!({
            "action": "in_progress",
            "workflow_job": {
                "id": 104275418234_i64,
                "node_id": "CR_kwDOIVVqXc8AAAAYz1nJeg",
                "run_id": 37610094675_i64,
                "run_attempt": 1,
                "name": "scan / Gitleaks",
                "workflow_name": "Secret Scanning",
                "head_branch": "renovate/amazon-aws-cli-2.x",
                "head_sha": "4be562610cec00928af0f80542cb5c959bedeeee",
                "status": "in_progress",
                "conclusion": null,
                "created_at": "2026-10-07T10:50:56Z",
                "started_at": "2026-10-07T10:51:16Z",
                "completed_at": null,
                "labels": ["github-arc-runner"],
                "runner_id": 77,
                "runner_name": "github-arc-runner-fxjbf-runner-bvk9h",
                "runner_group_id": 2,
                "runner_group_name": "kubernetes",
                "steps": [],
                "url": "https://api.github.com/repos/WMS-DEV/infra-charts/actions/jobs/104275418234",
                "html_url": "https://github.com/WMS-DEV/infra-charts/actions/runs/37610094675/job/104275418234",
                "run_url": "https://api.github.com/repos/WMS-DEV/infra-charts/actions/runs/37610094675",
                "check_run_url": "https://api.github.com/repos/WMS-DEV/infra-charts/check-runs/104275418234"
            },
            "repository": inner["repository"].clone(),
            "sender": inner["actor"].clone(),
        });

        let env: WorkflowJobEnvelope =
            serde_json::from_value(body.clone()).expect("real job delivery must parse");
        assert_eq!(env.action, WorkflowJobAction::InProgress);
        assert_eq!(env.workflow_job.id, 104275418234);
        assert_eq!(env.workflow_job.inner_name(), "Gitleaks");
        assert_eq!(env.repository.full_name, "WMS-DEV/infra-charts");

        // the inner payload alone cannot parse the delivery body
        assert!(serde_json::from_value::<WorkflowJob>(body).is_err());
    }

    #[test]
    fn all_run_actions_match_their_wire_names() {
        for (wire, expected) in [
            ("requested", WorkflowRunAction::Requested),
            ("in_progress", WorkflowRunAction::InProgress),
            ("completed", WorkflowRunAction::Completed),
        ] {
            let got: WorkflowRunAction = serde_json::from_value(json!(wire)).expect(wire);
            assert_eq!(got, expected, "{wire}");
        }
        assert!(serde_json::from_value::<WorkflowRunAction>(json!("queued")).is_err());
    }

    #[test]
    fn all_job_actions_match_their_wire_names() {
        for (wire, expected) in [
            ("queued", WorkflowJobAction::Queued),
            ("in_progress", WorkflowJobAction::InProgress),
            ("completed", WorkflowJobAction::Completed),
            ("waiting", WorkflowJobAction::Waiting),
        ] {
            let got: WorkflowJobAction = serde_json::from_value(json!(wire)).expect(wire);
            assert_eq!(got, expected, "{wire}");
        }
    }

    #[test]
    fn an_unknown_action_is_rejected_rather_than_silently_ingested() {
        let mut v = run_envelope_json();
        v.as_object_mut()
            .unwrap()
            .insert("action".into(), json!("exploded"));
        assert!(serde_json::from_value::<WorkflowRunEnvelope>(v).is_err());
    }
}
