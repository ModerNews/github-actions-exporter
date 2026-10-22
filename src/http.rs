pub mod handlers;
pub mod headers;

use axum::{
    Router,
    extract::{DefaultBodyLimit, FromRef},
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

use chrono::TimeDelta;

use crate::entry_map::EntityState;
use crate::metrics::Metrics;
use crate::scrape::Scraper;
use crate::signature::Verifier;

const MAX_BODY: usize = 25 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub verifier: Verifier,
    pub metrics: Metrics,
    pub entity_state: EntityState,
    pub entity_ttl: TimeDelta,
    pub scraper: Scraper,
}

impl AppState {
    pub fn new(verifier: Verifier, entity_ttl: TimeDelta) -> Self {
        Self {
            verifier,
            metrics: Metrics::default(),
            entity_state: EntityState::new(),
            entity_ttl,
            scraper: Scraper::new(),
        }
    }
}

impl FromRef<AppState> for Metrics {
    fn from_ref(state: &AppState) -> Self {
        state.metrics.clone()
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(handlers::health))
        .route("/api/v1/github", post(handlers::github_webhook))
        .route("/api/v1/metrics", get(handlers::get_metrics))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
