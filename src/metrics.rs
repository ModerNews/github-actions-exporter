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

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;

use crate::metrics::github_actions_exporter_deliveries_total::ExporterDeliveriesTotal;
use crate::metrics::github_actions_exporter_delivery_lag_seconds::ExporterDeliveryLagSeconds;
use crate::metrics::github_actions_exporter_last_delivery_timestamp_seconds::ExporterLastDeliveryTimestampSeconds;
use crate::metrics::github_actions_exporter_state_evictions_total::ExporterStateEvictionsTotal;
use crate::metrics::github_actions_job_duration_seconds::JobDurationSeconds;
use crate::metrics::github_actions_job_queue_seconds::JobQueueSeconds;
use crate::metrics::github_actions_jobs_in_flight::JobsInFlight;
use crate::metrics::github_actions_jobs_total::JobsTotal;
use crate::metrics::github_actions_queued_job_age_seconds_max::QueuedJobAgeSecondsMax;
use crate::metrics::github_actions_referenced_workflow_runs_total::ReferencedWorkflowRunsTotal;
use crate::metrics::github_actions_run_duration_seconds::RunDurationSeconds;
use crate::metrics::github_actions_runs_in_flight::RunsInFlight;
use crate::metrics::github_actions_runs_total::RunsTotal;
use crate::metrics::github_actions_workflow_last_completion_timestamp_seconds::WorkflowLastCompletionTimestampSeconds;

pub trait Ingester {
    fn ingest(&self, metrics: &Metrics) -> Result<(), Box<dyn Error>>;
}

pub trait Metric: Clone + prometheus_client::registry::Metric {
    const METRIC_KEY: &'static str;
    const METRIC_HELP: &'static str;

    fn new() -> Self;
}

#[macro_export]
macro_rules! metric {
    (
        $(#[$attr:meta])*
        $name:ident($inner:ty),
        key: $key:literal,
        help: $help:literal,
        new: $new:expr $(,)?
    ) => {
        $(#[$attr])*
        #[derive(Clone, Debug)]
        pub struct $name($inner);

        impl ::std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::prometheus_client::encoding::EncodeMetric for $name {
            fn encode(
                &self,
                encoder: ::prometheus_client::encoding::MetricEncoder,
            ) -> ::std::result::Result<(), ::std::fmt::Error> {
                ::prometheus_client::encoding::EncodeMetric::encode(&self.0, encoder)
            }

            fn metric_type(&self) -> ::prometheus_client::metrics::MetricType {
                ::prometheus_client::encoding::EncodeMetric::metric_type(&self.0)
            }

            fn is_empty(&self) -> bool {
                ::prometheus_client::encoding::EncodeMetric::is_empty(&self.0)
            }
        }

        impl $crate::metrics::Metric for $name {
            const METRIC_KEY: &'static str = $key;
            const METRIC_HELP: &'static str = $help;

            fn new() -> Self {
                Self($new)
            }
        }
    };
}

macro_rules! metrics {
    ($($field:ident: $ty:ty),+ $(,)?) => {
        #[derive(Clone)]
        pub struct Metrics {
            $(pub $field: $ty,)+
            registry: Arc<Registry>,
        }

        impl Default for Metrics {
            fn default() -> Self {
                let mut registry = Registry::default();

                $(
                    let $field = <$ty as Metric>::new();
                    registry.register(
                        <$ty as Metric>::METRIC_KEY,
                        <$ty as Metric>::METRIC_HELP,
                        $field.clone(),
                    );
                )+

                Self {
                    $($field,)+
                    registry: Arc::new(registry),
                }
            }
        }
    };
}

metrics! {
    jobs_total: JobsTotal,
    runs_total: RunsTotal,
    job_queue_seconds: JobQueueSeconds,
    job_duration_seconds: JobDurationSeconds,
    run_duration_seconds: RunDurationSeconds,
    referenced_workflow_runs_total: ReferencedWorkflowRunsTotal,

    jobs_in_flight: JobsInFlight,
    queued_job_age_seconds_max: QueuedJobAgeSecondsMax,
    runs_in_flight: RunsInFlight,
    workflow_last_completion_timestamp_seconds: WorkflowLastCompletionTimestampSeconds,

    exporter_deliveries_total: ExporterDeliveriesTotal,
    exporter_delivery_lag_seconds: ExporterDeliveryLagSeconds,
    exporter_state_evictions_total: ExporterStateEvictionsTotal,
    exporter_last_delivery_timestamp_seconds: ExporterLastDeliveryTimestampSeconds,
}

impl Metrics {
    pub fn encode(&self) -> Result<String, fmt::Error> {
        let mut out = String::new();
        encode(&mut out, &self.registry)?;
        Ok(out)
    }
}
