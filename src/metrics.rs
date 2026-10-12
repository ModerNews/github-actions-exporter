pub mod github_actions_exporter_deliveries_total;
pub mod github_actions_exporter_delivery_lag_seconds;
pub mod github_actions_exporter_last_delivery_timestamp_seconds;
pub mod github_actions_exporter_state_evictions_total;
pub mod github_actions_job_duration_seconds;
pub mod github_actions_job_queue_seconds;
pub mod github_actions_jobs_in_flight;
pub mod github_actions_jobs_total;
pub mod github_actions_queued_job_age_seconds_max;
pub mod github_actions_referenced_workflow_runs_total;
pub mod github_actions_run_duration_seconds;
pub mod github_actions_runs_in_flight;
pub mod github_actions_runs_total;
pub mod github_actions_workflow_last_completion_timestamp_seconds;

use std::fmt;
use std::sync::Arc;

use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;

use crate::metrics::github_actions_jobs_total::JobsTotal;

#[derive(Clone)]
pub struct Metrics {
    pub jobs_total: JobsTotal,
    registry: Arc<Registry>,
}

impl Default for Metrics {
    fn default() -> Self {
        let mut registry = Registry::default();

        let jobs_total = JobsTotal::default();
        registry.register(
            "github_actions_jobs",
            "Number of executed github actions jobs",
            jobs_total.clone(),
        );

        Self {
            jobs_total,
            registry: Arc::new(registry),
        }
    }
}

impl Metrics {
    pub fn encode(&self) -> Result<String, fmt::Error> {
        let mut out = String::new();
        encode(&mut out, &self.registry)?;
        Ok(out)
    }
}
