pub mod handlers;
pub mod headers;

use axum::{
    Router,
    extract::{DefaultBodyLimit, FromRef},
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

use crate::metrics::Metrics;
use crate::signature::Verifier;

const MAX_BODY: usize = 25 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub verifier: Verifier,
    pub metrics: Metrics,
}

impl AppState {
    pub fn new(verifier: Verifier) -> Self {
        Self {
            verifier,
            metrics: Metrics::default(),
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
