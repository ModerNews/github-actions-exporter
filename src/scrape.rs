//! Scrape side: evict, then recompute Tier 2 from the entity map.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};

use crate::entry_map::{EntityState, Entry, JobEntry, Rank, RunEntry, eviction_reason};
use crate::metrics::Metrics;
use crate::metrics::github_actions_exporter_state_evictions_total::ExporterStateEvictionsTotalLabels;
use crate::metrics::github_actions_jobs_in_flight::JobsInFlightLabels;
use crate::metrics::github_actions_queued_job_age_seconds_max::QueuedJobAgeSecondsMaxLabels;
use crate::metrics::github_actions_runs_in_flight::RunsInFlightLabels;

/// Serializes scrapes. Recomputation clears each Tier 2 family before
/// refilling it, so two overlapping scrapes would encode each other's
/// half-built state.
#[derive(Clone, Default)]
pub struct Scraper {
    lock: Arc<Mutex<()>>,
}

impl Scraper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(
        &self,
        state: &EntityState,
        metrics: &Metrics,
        now: DateTime<Utc>,
        ttl: TimeDelta,
    ) -> Result<String, fmt::Error> {
        let _guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);

        evict(state, metrics, now, ttl);
        recompute(state, metrics, now);

        metrics.encode()
    }
}

/// Rule 5. An entry still below terminal rank when it expired is a `completed`
/// that never arrived, so it is logged as well as counted.
fn evict(state: &EntityState, metrics: &Metrics, now: DateTime<Utc>, ttl: TimeDelta) {
    for (job_id, rank) in state.jobs.sweep(now, ttl) {
        if rank != Rank::Terminal {
            tracing::warn!(
                job_id,
                ?rank,
                "evicted a job whose completion never arrived"
            );
        }
        count_eviction(metrics, "job", eviction_reason(rank));
    }

    for ((run_id, run_attempt), rank) in state.runs.sweep(now, ttl) {
        if rank != Rank::Terminal {
            tracing::warn!(
                run_id,
                run_attempt,
                ?rank,
                "evicted a run whose completion never arrived"
            );
        }
        count_eviction(metrics, "run", eviction_reason(rank));
    }
}

fn count_eviction(metrics: &Metrics, entity: &str, reason: &str) {
    metrics
        .exporter_state_evictions_total
        .get_or_create(&ExporterStateEvictionsTotalLabels::new(entity, reason))
        .inc();
}

fn recompute(state: &EntityState, metrics: &Metrics, now: DateTime<Utc>) {
    // Cleared rather than overwritten: a label set that has left the map must
    // disappear, not report 0. Prometheus marks absent series stale, which is
    // what we want; zero-filling pins historical cardinality forever.
    metrics.jobs_in_flight.clear();
    metrics.queued_job_age_seconds_max.clear();
    metrics.runs_in_flight.clear();

    let mut jobs_in_flight: HashMap<JobsInFlightLabels, i64> = HashMap::new();
    let mut queued_age: HashMap<QueuedJobAgeSecondsMaxLabels, i64> = HashMap::new();

    state.jobs.for_each_in_flight(|entry: &JobEntry| {
        *jobs_in_flight
            .entry(JobsInFlightLabels::from(entry))
            .or_default() += 1;

        if entry.rank() == Rank::Pending {
            let age = (now - entry.created_at).num_seconds().max(0);
            let slot = queued_age
                .entry(QueuedJobAgeSecondsMaxLabels::from(entry))
                .or_default();
            *slot = (*slot).max(age);
        }
    });

    let mut runs_in_flight: HashMap<RunsInFlightLabels, i64> = HashMap::new();

    state.runs.for_each_in_flight(|entry: &RunEntry| {
        *runs_in_flight
            .entry(RunsInFlightLabels::from(entry))
            .or_default() += 1;
    });

    for (labels, count) in jobs_in_flight {
        metrics.jobs_in_flight.get_or_create(&labels).set(count);
    }

    // Untouched when nothing is queued: `0` would read as "nothing is
    // waiting" where absent correctly reads as "no queue to measure".
    for (labels, age) in queued_age {
        metrics
            .queued_job_age_seconds_max
            .get_or_create(&labels)
            .set(age);
    }

    for (labels, count) in runs_in_flight {
        metrics.runs_in_flight.get_or_create(&labels).set(count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhooks::workflow_status::WorkflowStatus;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).expect("valid timestamp")
    }

    fn job(status: WorkflowStatus, created_at: DateTime<Utc>, runner_label: &str) -> JobEntry {
        JobEntry {
            status,
            repository: "WMS-DEV/infra-charts".into(),
            workflow_name: "Secret Scanning".into(),
            runner_label: runner_label.into(),
            created_at,
        }
    }

    fn run(status: WorkflowStatus) -> RunEntry {
        RunEntry {
            status,
            repository: "WMS-DEV/infra-charts".into(),
            workflow_name: "Secret Scanning".into(),
            event: "pull_request".into(),
            created_at: at(0),
        }
    }

    /// Sample lines for one metric, HELP/TYPE excluded.
    fn samples(out: &str, name: &str) -> Vec<String> {
        let prefix = format!("{name}{{");
        out.lines()
            .filter(|line| line.starts_with(&prefix))
            .map(str::to_owned)
            .collect()
    }

    fn render(state: &EntityState, metrics: &Metrics, now: DateTime<Utc>) -> String {
        Scraper::new()
            .render(state, metrics, now, TimeDelta::hours(24))
            .expect("encoding cannot fail")
    }

    #[test]
    fn in_flight_counts_every_entry_below_terminal() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::Queued, at(0), "arc"));
        state
            .jobs
            .observe(2, job(WorkflowStatus::InProgress, at(0), "arc"));
        state.runs.observe((9, 1), run(WorkflowStatus::Requested));

        let out = render(&state, &metrics, at(10));

        assert_eq!(samples(&out, "github_actions_jobs_in_flight").len(), 2);
        assert_eq!(samples(&out, "github_actions_runs_in_flight").len(), 1);
    }

    /// Rule 3's corollary: the tombstone stays in the map, but it is not in
    /// flight and must not appear.
    #[test]
    fn a_tombstone_is_not_in_flight() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::Completed, at(0), "arc"));

        assert!(
            samples(
                &render(&state, &metrics, at(10)),
                "github_actions_jobs_in_flight"
            )
            .is_empty()
        );
    }

    /// `0` would read as "nothing is waiting"; absent reads as "no queue to
    /// measure", which is what Prometheus wants.
    #[test]
    fn queued_age_is_absent_when_nothing_is_queued() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::InProgress, at(0), "arc"));

        let out = render(&state, &metrics, at(10));
        assert!(samples(&out, "github_actions_queued_job_age_seconds_max").is_empty());
        assert!(!out.contains("queued_job_age_seconds_max"));
    }

    #[test]
    fn queued_age_is_the_max_over_queued_entries_only() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::Queued, at(900), "arc"));
        state
            .jobs
            .observe(2, job(WorkflowStatus::Waiting, at(500), "arc"));
        // older, but running — it is no longer queue time
        state
            .jobs
            .observe(3, job(WorkflowStatus::InProgress, at(0), "arc"));

        let out = render(&state, &metrics, at(1000));
        assert_eq!(
            samples(&out, "github_actions_queued_job_age_seconds_max"),
            [
                "github_actions_queued_job_age_seconds_max{repository=\"WMS-DEV/infra-charts\",runner_label=\"arc\"} 500"
            ]
        );
    }

    /// Clear-and-rebuild, not `set`: a label set that has left the map has to
    /// vanish so Prometheus marks it stale.
    #[test]
    fn a_departed_series_disappears_rather_than_reporting_zero() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::InProgress, at(0), "arc"));
        assert_eq!(
            samples(
                &render(&state, &metrics, at(10)),
                "github_actions_jobs_in_flight"
            )
            .len(),
            1
        );

        state
            .jobs
            .observe(1, job(WorkflowStatus::Completed, at(0), "arc"));
        let out = render(&state, &metrics, at(20));

        assert!(samples(&out, "github_actions_jobs_in_flight").is_empty());
        assert!(!out.contains("jobs_in_flight{"), "no zero-filled leftover");
    }

    /// Two runner pools must not collapse into one series.
    #[test]
    fn label_sets_are_counted_separately() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::Queued, at(0), "arc"));
        state
            .jobs
            .observe(2, job(WorkflowStatus::Queued, at(0), "hosted"));
        state
            .jobs
            .observe(3, job(WorkflowStatus::Queued, at(0), "hosted"));

        let mut got = samples(
            &render(&state, &metrics, at(10)),
            "github_actions_jobs_in_flight",
        );
        got.sort();

        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with("runner_label=\"arc\"} 1"), "{got:?}");
        assert!(got[1].ends_with("runner_label=\"hosted\"} 2"), "{got:?}");
    }

    /// Rule 5. Conflating the two makes the metric fire constantly during
    /// healthy operation, so only `lost` is alertable.
    #[test]
    fn evictions_are_split_by_the_rank_held() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::Completed, at(0), "arc"));
        state
            .jobs
            .observe(2, job(WorkflowStatus::Queued, at(0), "arc"));
        state.runs.observe((9, 1), run(WorkflowStatus::Requested));

        let out = Scraper::new()
            .render(&state, &metrics, at(100), TimeDelta::seconds(10))
            .unwrap();

        let mut got = samples(&out, "github_actions_exporter_state_evictions_total");
        got.sort();

        assert_eq!(
            got,
            [
                "github_actions_exporter_state_evictions_total{entity=\"job\",reason=\"completed\"} 1",
                "github_actions_exporter_state_evictions_total{entity=\"job\",reason=\"lost\"} 1",
                "github_actions_exporter_state_evictions_total{entity=\"run\",reason=\"lost\"} 1",
            ]
        );
        assert!(
            samples(&out, "github_actions_jobs_in_flight").is_empty(),
            "everything was evicted"
        );
    }

    /// Rule 4 — recomputed from scratch, so scraping twice cannot drift.
    #[test]
    fn repeated_renders_agree() {
        let state = EntityState::new();
        let metrics = Metrics::default();

        state
            .jobs
            .observe(1, job(WorkflowStatus::Queued, at(0), "arc"));
        state.runs.observe((9, 1), run(WorkflowStatus::InProgress));

        assert_eq!(
            render(&state, &metrics, at(10)),
            render(&state, &metrics, at(10))
        );
    }
}
