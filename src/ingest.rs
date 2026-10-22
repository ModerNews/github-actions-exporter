use std::error::Error;

use chrono::{DateTime, TimeDelta, Utc};

use crate::entry_map::{EntityState, JobEntry, RunEntry};
use crate::metrics::Metrics;
use crate::metrics::github_actions_exporter_delivery_lag_seconds::ExporterDeliveryLagSecondsLabels;
use crate::metrics::github_actions_job_duration_seconds::JobDurationSecondsLabels;
use crate::metrics::github_actions_job_queue_seconds::JobQueueSecondsLabels;
use crate::metrics::github_actions_jobs_total::JobsTotalLabels;
use crate::metrics::github_actions_referenced_workflow_runs_total::ReferencedWorkflowRunsTotalLabels;
use crate::metrics::github_actions_run_duration_seconds::RunDurationSecondsLabels;
use crate::metrics::github_actions_runs_total::RunsTotalLabels;
use crate::metrics::github_actions_workflow_last_completion_timestamp_seconds::WorkflowLastCompletionTimestampSecondsLabels;
use crate::webhooks::envelope::{WorkflowJobAction, WorkflowJobEnvelope, WorkflowRunEnvelope};
use crate::webhooks::event::WebhookEvent;

pub trait Ingester {
    fn ingest(&self, state: &EntityState, metrics: &Metrics) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    Processed { action: String },
    Ignored,
}

pub async fn process_webhook(
    event: WebhookEvent,
    body: &[u8],
    state: &EntityState,
    metrics: &Metrics,
) -> Result<Disposition, Box<dyn Error>> {
    match event {
        WebhookEvent::WorkflowRun => {
            let envelope = serde_json::from_slice::<WorkflowRunEnvelope>(body)?;
            let action = String::from(envelope.action.clone());
            envelope.ingest(state, metrics)?;
            Ok(Disposition::Processed { action })
        }
        WebhookEvent::WorkflowJob => {
            let envelope = serde_json::from_slice::<WorkflowJobEnvelope>(body)?;
            let action = String::from(envelope.action.clone());
            envelope.ingest(state, metrics)?;
            Ok(Disposition::Processed { action })
        }
        _ => Ok(Disposition::Ignored),
    }
}

fn lag_seconds(observed_at: DateTime<Utc>) -> f64 {
    (Utc::now() - observed_at).num_seconds().max(0) as f64
}

fn elapsed_seconds(span: TimeDelta) -> f64 {
    span.num_seconds().max(0) as f64
}

impl Ingester for WorkflowRunEnvelope {
    fn ingest(&self, state: &EntityState, metrics: &Metrics) -> Result<(), Box<dyn Error>> {
        let run = &self.workflow_run;

        tracing::info!(
            action = ?self.action,
            repo = %self.repository.full_name,
            "workflow_run"
        );

        metrics
            .exporter_last_delivery_timestamp_seconds
            .set(Utc::now().timestamp());

        metrics
            .exporter_delivery_lag_seconds
            .get_or_create(&ExporterDeliveryLagSecondsLabels::from(self))
            .observe(lag_seconds(run.updated_at));

        let outcome = state
            .runs
            .observe((run.id, run.run_attempt), RunEntry::from(self));

        if let Some(rejection) = outcome.rejection() {
            tracing::debug!(
                run_id = run.id,
                run_attempt = run.run_attempt,
                reason = rejection.as_str(),
                "delivery did not advance run state"
            );
            return Ok(());
        }

        if !outcome.is_terminal_transition() {
            return Ok(());
        }

        metrics
            .runs_total
            .get_or_create(&RunsTotalLabels::from(self))
            .inc();

        metrics
            .run_duration_seconds
            .get_or_create(&RunDurationSecondsLabels::from(self))
            .observe(elapsed_seconds(run.updated_at - run.run_started_at));

        for referenced in &run.referenced_workflows {
            metrics
                .referenced_workflow_runs_total
                .get_or_create(&ReferencedWorkflowRunsTotalLabels::from((self, referenced)))
                .inc();
        }

        let completed_at = run.updated_at.timestamp();
        let last = metrics
            .workflow_last_completion_timestamp_seconds
            .get_or_create(&WorkflowLastCompletionTimestampSecondsLabels::from(self));
        if completed_at > last.get() {
            last.set(completed_at);
        }

        Ok(())
    }
}

impl Ingester for WorkflowJobEnvelope {
    fn ingest(&self, state: &EntityState, metrics: &Metrics) -> Result<(), Box<dyn Error>> {
        let job = &self.workflow_job;

        tracing::info!(
            action = ?self.action,
            repo = %self.repository.full_name,
            "workflow_job"
        );

        metrics
            .exporter_last_delivery_timestamp_seconds
            .set(Utc::now().timestamp());

        metrics
            .exporter_delivery_lag_seconds
            .get_or_create(&ExporterDeliveryLagSecondsLabels::from(self))
            .observe(lag_seconds(match self.action {
                WorkflowJobAction::Queued | WorkflowJobAction::Waiting => job.created_at,
                WorkflowJobAction::InProgress => job.started_at,
                WorkflowJobAction::Completed => job.completed_at.unwrap_or(job.started_at),
            }));

        let outcome = state.jobs.observe(job.id, JobEntry::from(self));

        if let Some(rejection) = outcome.rejection() {
            tracing::debug!(
                job_id = job.id,
                reason = rejection.as_str(),
                "delivery did not advance job state"
            );
            return Ok(());
        }

        if !outcome.is_terminal_transition() {
            return Ok(());
        }

        metrics
            .jobs_total
            .get_or_create(&JobsTotalLabels::from(self))
            .inc();

        if job.queue_wait() > TimeDelta::zero() {
            metrics
                .job_queue_seconds
                .get_or_create(&JobQueueSecondsLabels::from(self))
                .observe(elapsed_seconds(job.queue_wait()));
        }

        if let Some(duration) = job.duration().filter(|d| *d > TimeDelta::zero()) {
            metrics
                .job_duration_seconds
                .get_or_create(&JobDurationSecondsLabels::from(self))
                .observe(elapsed_seconds(duration));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhooks::workflow_status::WorkflowStatus;
    use serde_json::{Value, json};

    /// Rebuilt from the inner-object fixture the way `envelope.rs`'s tests do.
    /// `status` drives the rank, so overriding it walks the ladder.
    fn envelope_json(status: WorkflowStatus) -> Value {
        let mut inner: Value =
            serde_json::from_str(include_str!("../tests/fixtures/workflow_run.json")).unwrap();
        let wire: String = status.into();
        inner["status"] = json!(wire);

        json!({
            "action": "completed",
            "workflow_run": inner.clone(),
            "repository": inner["repository"].clone(),
            "sender": inner["actor"].clone(),
        })
    }

    fn envelope(status: WorkflowStatus) -> WorkflowRunEnvelope {
        serde_json::from_value(envelope_json(status)).expect("real delivery body must parse")
    }

    fn samples(out: &str, name: &str) -> Vec<String> {
        let prefix = format!("{name}{{");
        out.lines()
            .filter(|line| line.starts_with(&prefix))
            .map(str::to_owned)
            .collect()
    }

    fn runs_total(metrics: &Metrics) -> Vec<String> {
        samples(&metrics.encode().unwrap(), "github_actions_runs_total")
    }

    /// The bug the entity map exists to fix: GitHub redelivers, and an
    /// unguarded counter would count the same run twice.
    #[test]
    fn a_redelivered_terminal_run_is_counted_once() {
        let state = EntityState::new();
        let metrics = Metrics::default();
        let env = envelope(WorkflowStatus::Completed);

        env.ingest(&state, &metrics).unwrap();
        env.ingest(&state, &metrics).unwrap();
        env.ingest(&state, &metrics).unwrap();

        let got = runs_total(&metrics);
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].ends_with("} 1"), "{got:?}");
    }

    /// Tier 1 is emitted on the transition, not per delivery.
    #[test]
    fn a_non_terminal_delivery_emits_no_tier_one() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        envelope(WorkflowStatus::Requested)
            .ingest(&state, &metrics)
            .unwrap();

        assert!(runs_total(&metrics).is_empty());
        assert!(
            !metrics
                .encode()
                .unwrap()
                .contains("run_duration_seconds_count"),
            "no duration observation before the run finishes"
        );
    }

    #[test]
    fn the_whole_ladder_counts_once_at_the_end() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        for status in [
            WorkflowStatus::Requested,
            WorkflowStatus::InProgress,
            WorkflowStatus::Completed,
        ] {
            envelope(status).ingest(&state, &metrics).unwrap();
        }

        let got = runs_total(&metrics);
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].ends_with("} 1"), "{got:?}");
    }

    /// Tier 3 describes deliveries, not entities, so it records even the
    /// deliveries the entity map rejects — otherwise a redelivery storm looks
    /// like an idle org.
    #[test]
    fn tier_three_records_a_rejected_delivery() {
        let state = EntityState::new();
        let metrics = Metrics::default();
        let env = envelope(WorkflowStatus::Completed);

        env.ingest(&state, &metrics).unwrap();
        env.ingest(&state, &metrics).unwrap();

        let out = metrics.encode().unwrap();
        assert_eq!(
            samples(&out, "github_actions_exporter_delivery_lag_seconds_count")
                .first()
                .map(|line| line.ends_with("} 2")),
            Some(true),
            "both deliveries measured: {out}"
        );
    }

    /// An event we do not model is a no-op, not an error — and it must be
    /// distinguishable from a processed one, or an org-wide gap reads as an
    /// idle org.
    #[test]
    fn an_unmodelled_event_is_ignored_not_failed() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        let result = futures_lite_block(process_webhook(
            WebhookEvent::Other("push".into()),
            b"not even json",
            &state,
            &metrics,
        ));

        assert_eq!(result.unwrap(), Disposition::Ignored);
        assert!(!metrics.encode().unwrap().contains("github_actions_runs"));
    }

    /// The `action` label is only knowable after the body parses, which is why
    /// the disposition carries it back out.
    #[test]
    fn a_processed_delivery_reports_its_action() {
        let state = EntityState::new();
        let metrics = Metrics::default();
        let body = serde_json::to_vec(&envelope_json(WorkflowStatus::Completed)).unwrap();

        let disposition = futures_lite_block(process_webhook(
            WebhookEvent::WorkflowRun,
            &body,
            &state,
            &metrics,
        ))
        .expect("body parses");

        assert_eq!(
            disposition,
            Disposition::Processed {
                action: "completed".into()
            }
        );
    }

    /// A malformed body for an event we *do* model is an error, not `Ignored`.
    #[test]
    fn an_unparseable_modelled_event_is_an_error() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        assert!(
            futures_lite_block(process_webhook(
                WebhookEvent::WorkflowRun,
                b"{}",
                &state,
                &metrics
            ))
            .is_err()
        );
    }

    /// The closed set `metrics.md` enumerates for the `outcome` label.
    #[test]
    fn delivery_outcomes_match_the_documented_values() {
        use crate::metrics::github_actions_exporter_deliveries_total::DeliveryOutcome;

        assert_eq!(
            [
                DeliveryOutcome::Processed.as_str(),
                DeliveryOutcome::Ignored.as_str(),
                DeliveryOutcome::RejectedSignature.as_str(),
                DeliveryOutcome::RejectedHeader.as_str(),
                DeliveryOutcome::ParseError.as_str(),
            ],
            [
                "processed",
                "ignored",
                "rejected_signature",
                "rejected_header",
                "parse_error"
            ]
        );
    }

    /// `process_webhook` is `async` but never awaits, so a trivial executor is
    /// enough to drive it in tests.
    fn futures_lite_block<F: std::future::Future>(future: F) -> F::Output {
        let mut future = Box::pin(future);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("process_webhook awaited unexpectedly"),
        }
    }
}
