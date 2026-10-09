use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::Ingester;
use crate::webhooks::workflow_conclusion::WorkflowConclusion;
use crate::webhooks::workflow_status::WorkflowStatus;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowJob {
    pub id: i64,
    pub node_id: String,
    pub run_id: i64,
    pub run_attempt: i64,
    pub name: String,
    pub workflow_name: Option<String>,
    pub head_branch: Option<String>,
    pub head_sha: String,
    pub status: WorkflowStatus,
    pub conclusion: Option<WorkflowConclusion>,
    pub created_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub labels: Vec<String>,
    pub runner_id: Option<i64>,
    pub runner_name: Option<String>,
    pub runner_group_id: Option<i64>,
    pub runner_group_name: Option<String>,
    pub steps: Vec<WorkflowStep>,
    pub url: String,
    pub html_url: String,
    pub run_url: String,
    pub check_run_url: String,
}

impl Ingester for WorkflowJob {
    fn ingest(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(?self);
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub number: i64,
    pub status: WorkflowStatus,
    pub conclusion: Option<WorkflowConclusion>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl WorkflowJob {
    pub fn queue_wait(&self) -> chrono::TimeDelta {
        self.started_at - self.created_at
    }

    pub fn duration(&self) -> Option<chrono::TimeDelta> {
        self.completed_at.map(|done| done - self.started_at)
    }

    pub fn inner_name(&self) -> &str {
        self.name
            .split_once(" / ")
            .map_or(self.name.as_str(), |(_, inner)| inner)
    }

    pub fn caller_job_key(&self) -> Option<&str> {
        self.name.split_once(" / ").map(|(key, _)| key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn completed_job() -> Value {
        json!({
            "id": 104275418234_i64,
            "node_id": "CR_kwDOIVVqXc8AAAAYz1nJeg",
            "run_id": 37610094675_i64,
            "run_attempt": 1,
            "name": "scan / Gitleaks",
            "workflow_name": "Secret Scanning",
            "head_branch": "renovate/amazon-aws-cli-2.x",
            "head_sha": "4be562610cec00928af0f80542cb5c959bedeeee",
            "status": "completed",
            "conclusion": "success",
            "created_at": "2026-10-07T10:50:56Z",
            "started_at": "2026-10-07T10:51:16Z",
            "completed_at": "2026-10-07T10:51:48Z",
            "labels": ["github-arc-runner"],
            "runner_id": 77,
            "runner_name": "github-arc-runner-fxjbf-runner-bvk9h",
            "runner_group_id": 2,
            "runner_group_name": "kubernetes",
            "steps": [
                {
                    "name": "Set up job",
                    "number": 1,
                    "status": "completed",
                    "conclusion": "success",
                    "started_at": "2026-10-07T10:51:16Z",
                    "completed_at": "2026-10-07T10:51:18Z"
                },
                {
                    "name": "Run Gitleaks",
                    "number": 2,
                    "status": "in_progress",
                    "conclusion": null,
                    "started_at": "2026-10-07T10:51:18Z",
                    "completed_at": null
                },
                {
                    "name": "Post job cleanup",
                    "number": 3,
                    "status": "queued",
                    "conclusion": null,
                    "started_at": null,
                    "completed_at": null
                }
            ],
            "url": "https://api.github.com/repos/WMS-DEV/infra-charts/actions/jobs/104275418234",
            "html_url": "https://github.com/WMS-DEV/infra-charts/actions/runs/37610094675/job/104275418234",
            "run_url": "https://api.github.com/repos/WMS-DEV/infra-charts/actions/runs/37610094675",
            "check_run_url": "https://api.github.com/repos/WMS-DEV/infra-charts/check-runs/104275418234"
        })
    }

    fn parse(label: &str, v: &Value) -> WorkflowJob {
        serde_json::from_value(v.clone())
            .unwrap_or_else(|e| panic!("{label} must deserialize, got: {e}"))
    }

    fn job() -> WorkflowJob {
        parse("completed job", &completed_job())
    }

    #[test]
    fn deserializes_a_completed_job() {
        let j = job();
        assert_eq!(j.id, 104275418234);
        assert_eq!(j.run_id, 37610094675);
        assert_eq!(j.run_attempt, 1);
        assert_eq!(j.status, WorkflowStatus::Completed);
        assert_eq!(j.conclusion, Some(WorkflowConclusion::Success));
        assert_eq!(j.head_sha.len(), 40);
    }

    #[test]
    fn ids_do_not_fit_i32() {
        let j = job();
        assert!(j.id > i32::MAX as i64);
        assert!(j.run_id > i32::MAX as i64);
    }

    #[test]
    fn queue_wait_is_the_runner_wait() {
        assert_eq!(job().queue_wait().num_seconds(), 20);
    }

    #[test]
    fn duration_spans_started_to_completed() {
        assert_eq!(job().duration().map(|d| d.num_seconds()), Some(32));
    }

    #[test]
    fn duration_is_none_until_completed() {
        let mut v = completed_job();
        let obj = v.as_object_mut().unwrap();
        obj.insert("status".into(), json!("in_progress"));
        obj.insert("conclusion".into(), Value::Null);
        obj.insert("completed_at".into(), Value::Null);
        let j = parse("in-progress job", &v);
        assert!(j.conclusion.is_none());
        assert!(j.completed_at.is_none());
        assert!(j.duration().is_none());
        assert_eq!(j.queue_wait().num_seconds(), 20);
    }

    #[test]
    fn name_splits_into_caller_key_and_inner_name() {
        let j = job();
        assert_eq!(j.name, "scan / Gitleaks");
        assert_eq!(j.inner_name(), "Gitleaks");
        assert_eq!(j.caller_job_key(), Some("scan"));
    }

    #[test]
    fn unprefixed_name_is_unchanged() {
        let mut v = completed_job();
        v.as_object_mut()
            .unwrap()
            .insert("name".into(), json!("build"));
        let j = parse("plain job", &v);
        assert_eq!(j.inner_name(), "build");
        assert_eq!(j.caller_job_key(), None);
    }

    #[test]
    fn only_the_caller_prefix_is_stripped() {
        let mut v = completed_job();
        v.as_object_mut()
            .unwrap()
            .insert("name".into(), json!("scan / Lint / Format"));
        let j = parse("nested name", &v);
        assert_eq!(j.caller_job_key(), Some("scan"));
        assert_eq!(j.inner_name(), "Lint / Format");
    }

    #[test]
    fn workflow_name_is_the_callers_and_nullable() {
        assert_eq!(job().workflow_name.as_deref(), Some("Secret Scanning"));
        let mut v = completed_job();
        v.as_object_mut()
            .unwrap()
            .insert("workflow_name".into(), Value::Null);
        assert!(parse("null workflow_name", &v).workflow_name.is_none());
    }

    #[test]
    fn runner_identity_is_captured_but_ephemeral() {
        let j = job();
        assert_eq!(j.labels, ["github-arc-runner"]);
        assert_eq!(
            j.runner_name.as_deref(),
            Some("github-arc-runner-fxjbf-runner-bvk9h")
        );
        assert_eq!(j.runner_group_name.as_deref(), Some("kubernetes"));
        assert_eq!(j.runner_id, Some(77));
    }

    #[test]
    fn runner_fields_are_all_nullable() {
        let mut v = completed_job();
        let obj = v.as_object_mut().unwrap();
        for k in [
            "runner_id",
            "runner_name",
            "runner_group_id",
            "runner_group_name",
        ] {
            obj.insert(k.into(), Value::Null);
        }
        obj.insert("head_branch".into(), Value::Null);
        let j = parse("job without a runner", &v);
        assert!(j.runner_id.is_none());
        assert!(j.runner_name.is_none());
        assert!(j.runner_group_id.is_none());
        assert!(j.runner_group_name.is_none());
        assert!(j.head_branch.is_none());
    }

    #[test]
    fn steps_cover_completed_in_progress_and_queued_together() {
        let steps = job().steps;
        assert_eq!(steps.len(), 3);

        assert_eq!(steps[0].status, WorkflowStatus::Completed);
        assert_eq!(steps[0].conclusion, Some(WorkflowConclusion::Success));
        assert!(steps[0].started_at.is_some() && steps[0].completed_at.is_some());

        assert_eq!(steps[1].status, WorkflowStatus::InProgress);
        assert!(steps[1].conclusion.is_none());
        assert!(steps[1].started_at.is_some() && steps[1].completed_at.is_none());

        assert_eq!(steps[2].status, WorkflowStatus::Queued);
        assert!(steps[2].conclusion.is_none());
        assert!(steps[2].started_at.is_none() && steps[2].completed_at.is_none());

        assert_eq!(steps[0].number, 1);
        assert_eq!(steps[2].name, "Post job cleanup");
    }

    #[test]
    fn steps_may_be_empty() {
        let mut v = completed_job();
        v.as_object_mut().unwrap().insert("steps".into(), json!([]));
        assert!(parse("job with no steps", &v).steps.is_empty());
    }

    #[test]
    fn every_job_status_parses() {
        for (wire, expected) in [
            ("queued", WorkflowStatus::Queued),
            ("in_progress", WorkflowStatus::InProgress),
            ("completed", WorkflowStatus::Completed),
            ("waiting", WorkflowStatus::Waiting),
        ] {
            let mut v = completed_job();
            v.as_object_mut()
                .unwrap()
                .insert("status".into(), json!(wire));
            assert_eq!(parse(wire, &v).status, expected, "{wire}");
        }
    }

    #[test]
    fn conclusions_beyond_the_job_schema_are_accepted() {
        for (wire, expected) in [
            ("success", WorkflowConclusion::Success),
            ("failure", WorkflowConclusion::Failure),
            ("cancelled", WorkflowConclusion::Cancelled),
            ("skipped", WorkflowConclusion::Skipped),
            ("timed_out", WorkflowConclusion::TimedOut),
            ("neutral", WorkflowConclusion::Neutral),
            ("action_required", WorkflowConclusion::ActionRequired),
        ] {
            let mut v = completed_job();
            v.as_object_mut()
                .unwrap()
                .insert("conclusion".into(), json!(wire));
            assert_eq!(parse(wire, &v).conclusion, Some(expected), "{wire}");
        }
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let mut v = completed_job();
        v.as_object_mut()
            .unwrap()
            .insert("some_future_field".into(), json!({"a": 1}));
        let _ = parse("job + unknown field", &v);

        let mut v = completed_job();
        v["steps"][0]
            .as_object_mut()
            .unwrap()
            .insert("future_step_field".into(), json!(true));
        let _ = parse("step + unknown field", &v);
    }
}
