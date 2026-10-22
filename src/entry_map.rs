//! Entity state.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use chrono::{DateTime, TimeDelta, Utc};

use crate::webhooks::envelope::{WorkflowJobEnvelope, WorkflowRunEnvelope};
use crate::webhooks::workflow_status::WorkflowStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    Pending,
    InProgress,
    Terminal,
}

impl From<&WorkflowStatus> for Rank {
    fn from(value: &WorkflowStatus) -> Self {
        match value {
            WorkflowStatus::Requested | WorkflowStatus::Queued | WorkflowStatus::Waiting => {
                Self::Pending
            }
            WorkflowStatus::InProgress => Self::InProgress,
            WorkflowStatus::Completed => Self::Terminal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Stale,
    AlreadyTerminal,
}

impl Rejection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::AlreadyTerminal => "already_terminal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Terminal,
    Recorded,
    Rejected(Rejection),
}

impl Outcome {
    pub fn is_terminal_transition(&self) -> bool {
        matches!(self, Self::Terminal)
    }

    pub fn rejection(&self) -> Option<Rejection> {
        match self {
            Self::Rejected(rejection) => Some(*rejection),
            _ => None,
        }
    }
}

pub trait Entry {
    fn rank(&self) -> Rank;
    fn created_at(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone)]
pub struct JobEntry {
    pub status: WorkflowStatus,
    pub repository: String,
    pub workflow_name: String,
    pub runner_label: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RunEntry {
    pub status: WorkflowStatus,
    pub repository: String,
    pub workflow_name: String,
    pub event: String,
    pub created_at: DateTime<Utc>,
}

impl Entry for JobEntry {
    fn rank(&self) -> Rank {
        Rank::from(&self.status)
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

impl Entry for RunEntry {
    fn rank(&self) -> Rank {
        Rank::from(&self.status)
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

pub fn eviction_reason(rank: Rank) -> &'static str {
    match rank {
        Rank::Terminal => "completed",
        Rank::Pending | Rank::InProgress => "lost",
    }
}

pub struct Store<K, E> {
    entries: Arc<RwLock<HashMap<K, E>>>,
}

impl<K, E> Clone for Store<K, E> {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
        }
    }
}

impl<K, E> Default for Store<K, E> {
    fn default() -> Self {
        Self {
            entries: Arc::default(),
        }
    }
}

impl<K: Eq + Hash, E: Entry> Store<K, E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&self, key: K, entry: E) -> Outcome {
        let mut entries = self.write();
        let incoming = entry.rank();

        match entries.get(&key).map(|stored| stored.rank()) {
            Some(Rank::Terminal) => Outcome::Rejected(Rejection::AlreadyTerminal),

            Some(stored) if incoming < stored => Outcome::Rejected(Rejection::Stale),

            _ => {
                entries.insert(key, entry);
                match incoming {
                    Rank::Terminal => Outcome::Terminal,
                    _ => Outcome::Recorded,
                }
            }
        }
    }

    pub fn sweep(&self, now: DateTime<Utc>, ttl: TimeDelta) -> Vec<(K, Rank)> {
        self.write()
            .extract_if(|_, entry| now - entry.created_at() >= ttl)
            .map(|(key, entry)| (key, entry.rank()))
            .collect()
    }

    pub fn for_each_in_flight(&self, mut f: impl FnMut(&E)) {
        for entry in self.read().values() {
            if entry.rank() != Rank::Terminal {
                f(entry);
            }
        }
    }

    fn read(&self) -> RwLockReadGuard<'_, HashMap<K, E>> {
        self.entries.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<K, E>> {
        self.entries.write().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Clone, Default)]
pub struct EntityState {
    pub jobs: Store<i64, JobEntry>,
    pub runs: Store<(i64, i64), RunEntry>,
}

impl EntityState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<&WorkflowJobEnvelope> for JobEntry {
    fn from(value: &WorkflowJobEnvelope) -> Self {
        let job = &value.workflow_job;
        Self {
            status: job.status.clone(),
            repository: value.repository.full_name.clone(),
            workflow_name: job.workflow_name.clone().unwrap_or_default(),
            runner_label: job.runner_label(),
            created_at: job.created_at,
        }
    }
}

impl From<&WorkflowRunEnvelope> for RunEntry {
    fn from(value: &WorkflowRunEnvelope) -> Self {
        let run = &value.workflow_run;
        Self {
            status: run.status.clone(),
            repository: value.repository.full_name.clone(),
            workflow_name: run.name.clone(),
            event: run.event.clone(),
            created_at: run.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).expect("valid timestamp")
    }

    fn job(status: WorkflowStatus) -> JobEntry {
        job_created_at(status, at(0))
    }

    fn job_created_at(status: WorkflowStatus, created_at: DateTime<Utc>) -> JobEntry {
        JobEntry {
            status,
            repository: "WMS-DEV/infra-charts".into(),
            workflow_name: "Secret Scanning".into(),
            runner_label: "github-arc-runner".into(),
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

    fn status_at(rank: Rank) -> WorkflowStatus {
        match rank {
            Rank::Pending => WorkflowStatus::Queued,
            Rank::InProgress => WorkflowStatus::InProgress,
            Rank::Terminal => WorkflowStatus::Completed,
        }
    }

    /// Rebuilt from the inner-object fixture the same way `envelope.rs`'s
    /// tests do, so the conversion is exercised against a real payload.
    fn real_run_envelope() -> WorkflowRunEnvelope {
        let inner: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/workflow_run.json")).unwrap();
        serde_json::from_value(serde_json::json!({
            "action": "completed",
            "workflow_run": inner.clone(),
            "repository": inner["repository"].clone(),
            "sender": inner["actor"].clone(),
        }))
        .expect("real delivery body must parse")
    }

    /// Every field here is a Tier 2 label at scrape time, when the payload is
    /// long gone — a mis-wired one is invisible until a dashboard is wrong.
    #[test]
    fn a_run_envelope_converts_to_an_entry() {
        let entry = RunEntry::from(&real_run_envelope());

        assert_eq!(entry.repository, "WMS-DEV/infra-charts");
        assert_eq!(entry.workflow_name, "Secret Scanning");
        assert_eq!(entry.status, WorkflowStatus::Completed);
        assert_eq!(entry.rank(), Rank::Terminal);
    }

    /// The `event` label is the **trigger**, not the delivery type. Typing it
    /// as `WebhookEvent` would compile and emit `event="workflow_run"` on
    /// every series.
    #[test]
    fn the_event_label_is_the_trigger_not_the_delivery_type() {
        assert_eq!(RunEntry::from(&real_run_envelope()).event, "pull_request");
    }

    /// `created_at`, not `run_started_at`: the TTL is a lifetime bound, and
    /// the two are verified identical on every run observed anyway.
    #[test]
    fn a_run_entry_ages_from_created_at() {
        let env = real_run_envelope();
        assert_eq!(
            RunEntry::from(&env).created_at(),
            env.workflow_run.created_at
        );
    }

    /// `derive(Ord)` reads declaration order, so a reordered enum would
    /// silently invert rules 2 and 3. This is the guard.
    #[test]
    fn rank_order_is_the_ladder() {
        assert!(Rank::Pending < Rank::InProgress);
        assert!(Rank::InProgress < Rank::Terminal);
    }

    /// Exhaustive over `WorkflowStatus`: a status GitHub adds later fails to
    /// compile in `Rank::from` rather than silently landing on `Pending`.
    #[test]
    fn every_status_has_a_rank() {
        for (status, expected) in [
            (WorkflowStatus::Requested, Rank::Pending),
            (WorkflowStatus::Queued, Rank::Pending),
            (WorkflowStatus::Waiting, Rank::Pending),
            (WorkflowStatus::InProgress, Rank::InProgress),
            (WorkflowStatus::Completed, Rank::Terminal),
        ] {
            assert_eq!(Rank::from(&status), expected, "{status:?}");
        }
    }

    /// The full decision table from `docs/entity-state-tests.md`.
    #[test]
    fn the_transition_table_holds() {
        let cases = [
            (None, Rank::Pending, Outcome::Recorded),
            (None, Rank::InProgress, Outcome::Recorded),
            (None, Rank::Terminal, Outcome::Terminal),
            (Some(Rank::Pending), Rank::Pending, Outcome::Recorded),
            (Some(Rank::Pending), Rank::InProgress, Outcome::Recorded),
            (Some(Rank::Pending), Rank::Terminal, Outcome::Terminal),
            (
                Some(Rank::InProgress),
                Rank::Pending,
                Outcome::Rejected(Rejection::Stale),
            ),
            (Some(Rank::InProgress), Rank::InProgress, Outcome::Recorded),
            (Some(Rank::InProgress), Rank::Terminal, Outcome::Terminal),
            (
                Some(Rank::Terminal),
                Rank::Pending,
                Outcome::Rejected(Rejection::AlreadyTerminal),
            ),
            (
                Some(Rank::Terminal),
                Rank::InProgress,
                Outcome::Rejected(Rejection::AlreadyTerminal),
            ),
            (
                Some(Rank::Terminal),
                Rank::Terminal,
                Outcome::Rejected(Rejection::AlreadyTerminal),
            ),
        ];

        for (stored, incoming, expected) in cases {
            let store = Store::new();
            if let Some(stored) = stored {
                store.observe(1, job(status_at(stored)));
            }
            assert_eq!(
                store.observe(1, job(status_at(incoming))),
                expected,
                "stored={stored:?} incoming={incoming:?}"
            );
        }
    }

    #[test]
    fn a_job_lifecycle_transitions_to_terminal_exactly_once() {
        let store = Store::new();
        let outcomes: Vec<_> = [
            WorkflowStatus::Queued,
            WorkflowStatus::InProgress,
            WorkflowStatus::Completed,
        ]
        .into_iter()
        .map(|status| store.observe(7, job(status)))
        .collect();

        assert_eq!(
            outcomes,
            [Outcome::Recorded, Outcome::Recorded, Outcome::Terminal]
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| o.is_terminal_transition())
                .count(),
            1
        );
    }

    #[test]
    fn a_run_lifecycle_has_no_queued_stage() {
        let store = Store::new();
        let outcomes: Vec<_> = [
            WorkflowStatus::Requested,
            WorkflowStatus::InProgress,
            WorkflowStatus::Completed,
        ]
        .into_iter()
        .map(|status| store.observe((7, 1), run(status)))
        .collect();

        assert_eq!(
            outcomes,
            [Outcome::Recorded, Outcome::Recorded, Outcome::Terminal]
        );
    }

    /// Rule 1. Idempotency comes from the map, not from a delivery-GUID cache,
    /// so a duplicate is a no-op by construction.
    #[test]
    fn duplicate_deliveries_do_not_accumulate() {
        let store = Store::new();
        assert_eq!(
            store.observe(7, job(WorkflowStatus::Queued)),
            Outcome::Recorded
        );
        assert_eq!(
            store.observe(7, job(WorkflowStatus::Queued)),
            Outcome::Recorded
        );
        assert_eq!(
            store.observe(7, job(WorkflowStatus::Queued)),
            Outcome::Recorded
        );

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 1, "one entity, one entry");
    }

    /// The double-count bug rule 3 exists to prevent.
    #[test]
    fn a_redelivered_completed_does_not_transition_twice() {
        let store = Store::new();
        assert!(
            store
                .observe(7, job(WorkflowStatus::Completed))
                .is_terminal_transition()
        );
        assert!(
            !store
                .observe(7, job(WorkflowStatus::Completed))
                .is_terminal_transition()
        );
    }

    /// Equal rank is accepted and updates the entry — that is the only way
    /// `queued` -> `waiting` can register — but it is not a transition.
    #[test]
    fn waiting_updates_the_status_without_transitioning() {
        let store = Store::new();
        store.observe(7, job(WorkflowStatus::Queued));
        assert_eq!(
            store.observe(7, job(WorkflowStatus::Waiting)),
            Outcome::Recorded
        );

        let mut seen = None;
        store.for_each_in_flight(|e| seen = Some(e.status.clone()));
        assert_eq!(seen, Some(WorkflowStatus::Waiting));
    }

    /// Rule 2, and the reason tombstones are kept: without the entry, a late
    /// `in_progress` would find nothing and recreate the job as in-flight.
    #[test]
    fn a_late_in_progress_cannot_resurrect_a_finished_job() {
        let store = Store::new();
        store.observe(7, job(WorkflowStatus::Completed));
        assert_eq!(
            store.observe(7, job(WorkflowStatus::InProgress)),
            Outcome::Rejected(Rejection::AlreadyTerminal)
        );

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 0);
    }

    /// First sighting may be mid-ladder; the earlier delivery arriving later
    /// must not pull it back.
    #[test]
    fn a_late_queued_cannot_pull_a_running_job_back() {
        let store = Store::new();
        assert_eq!(
            store.observe(7, job(WorkflowStatus::InProgress)),
            Outcome::Recorded
        );
        assert_eq!(
            store.observe(7, job(WorkflowStatus::Queued)),
            Outcome::Rejected(Rejection::Stale)
        );
    }

    /// Restart, or a delivery for an entity whose ladder we never saw.
    #[test]
    fn a_terminal_first_sighting_transitions_and_leaves_a_tombstone() {
        let store = Store::new();
        assert_eq!(
            store.observe(7, job(WorkflowStatus::Completed)),
            Outcome::Terminal
        );

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 0, "the entry exists, but is not in flight");

        assert_eq!(
            store.observe(7, job(WorkflowStatus::Completed)),
            Outcome::Rejected(Rejection::AlreadyTerminal),
            "the tombstone is there to be found"
        );
    }

    #[test]
    fn the_walk_counts_only_entries_below_terminal() {
        let store = Store::new();
        store.observe(1, job(WorkflowStatus::Queued));
        store.observe(2, job(WorkflowStatus::InProgress));
        store.observe(3, job(WorkflowStatus::Completed));

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 2);
    }

    /// Rule 4 — the walk is a pure function of the map, so repeated scrapes
    /// cannot drift.
    #[test]
    fn repeated_walks_agree() {
        let store = Store::new();
        store.observe(1, job(WorkflowStatus::Queued));
        store.observe(2, job(WorkflowStatus::InProgress));

        let count = || {
            let mut n = 0;
            store.for_each_in_flight(|_| n += 1);
            n
        };
        assert_eq!(count(), count());
    }

    /// A re-run is a new entity: same `run_id`, new `run_attempt`, so both can
    /// be in flight and both may transition.
    #[test]
    fn a_retry_is_a_separate_entity() {
        let store = Store::new();
        assert_eq!(
            store.observe((7, 1), run(WorkflowStatus::Completed)),
            Outcome::Terminal
        );
        assert_eq!(
            store.observe((7, 2), run(WorkflowStatus::Requested)),
            Outcome::Recorded
        );

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 1, "attempt 1 is a tombstone, attempt 2 is live");
    }

    /// Rule 5 — the rank held at eviction is what the eviction means.
    #[test]
    fn eviction_reason_splits_on_the_rank_held() {
        let store = Store::new();
        store.observe(1, job_created_at(WorkflowStatus::Queued, at(0)));
        store.observe(2, job_created_at(WorkflowStatus::InProgress, at(0)));
        store.observe(3, job_created_at(WorkflowStatus::Completed, at(0)));

        let mut reasons: Vec<_> = store
            .sweep(at(100), TimeDelta::seconds(10))
            .into_iter()
            .map(|(_, rank)| eviction_reason(rank))
            .collect();
        reasons.sort_unstable();

        assert_eq!(reasons, ["completed", "lost", "lost"]);
    }

    /// The sweep returns keys so a `lost` eviction can name the entity whose
    /// `completed` never arrived — a bare rank would say only that something
    /// was lost.
    #[test]
    fn the_sweep_names_what_it_evicted() {
        let store = Store::new();
        store.observe(41, job_created_at(WorkflowStatus::Queued, at(0)));
        store.observe(42, job_created_at(WorkflowStatus::Queued, at(500)));

        assert_eq!(
            store.sweep(at(100), TimeDelta::seconds(10)),
            [(41, Rank::Pending)],
            "only the expired entry, and it is identifiable"
        );
    }

    /// `metrics.md` warns that a derive will not honour serde's `rename_all`,
    /// so label and log values go through `as_str`.
    #[test]
    fn rejection_reasons_are_snake_case() {
        assert_eq!(Rejection::Stale.as_str(), "stale");
        assert_eq!(Rejection::AlreadyTerminal.as_str(), "already_terminal");
    }

    #[test]
    fn a_rejection_is_reportable_and_a_transition_is_not() {
        let store = Store::new();
        store.observe(7, job(WorkflowStatus::Completed));

        assert_eq!(
            store.observe(7, job(WorkflowStatus::Completed)).rejection(),
            Some(Rejection::AlreadyTerminal)
        );
        assert_eq!(
            store.observe(8, job(WorkflowStatus::Queued)).rejection(),
            None
        );
    }

    #[test]
    fn an_entry_inside_the_ttl_is_retained() {
        let store = Store::new();
        store.observe(1, job_created_at(WorkflowStatus::Queued, at(0)));

        assert!(store.sweep(at(5), TimeDelta::seconds(10)).is_empty());

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 1);
    }

    /// The sweep must judge every entry against the instant it is given, not
    /// against the wall clock — otherwise one sweep spans many `now`s and no
    /// boundary is testable.
    #[test]
    fn the_sweep_uses_the_clock_it_is_given() {
        let store = Store::new();
        store.observe(1, job_created_at(WorkflowStatus::Queued, at(0)));

        assert!(
            store.sweep(at(0), TimeDelta::hours(24)).is_empty(),
            "age 0 against the passed clock, however old in wall-clock terms"
        );
    }

    /// Pins the accepted limitation rather than leaving it to be rediscovered:
    /// duplicate protection lasts exactly as long as the tombstone, so a
    /// redelivery after the TTL double-counts by design.
    #[test]
    fn duplicate_protection_ends_with_the_tombstone() {
        let store = Store::new();
        let completed = || job_created_at(WorkflowStatus::Completed, at(0));

        assert_eq!(store.observe(7, completed()), Outcome::Terminal);
        assert_eq!(store.sweep(at(100), TimeDelta::seconds(10)).len(), 1);
        assert_eq!(
            store.observe(7, completed()),
            Outcome::Terminal,
            "the tombstone is gone, so this is indistinguishable from a first sighting"
        );
    }

    #[test]
    fn clones_share_one_map() {
        let store: Store<i64, JobEntry> = Store::new();
        let handle = store.clone();
        handle.observe(7, job(WorkflowStatus::Queued));

        let mut in_flight = 0;
        store.for_each_in_flight(|_| in_flight += 1);
        assert_eq!(in_flight, 1);
    }
}
