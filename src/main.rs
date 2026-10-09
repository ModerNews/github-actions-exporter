mod errors;
mod webhooks;

use std::error::Error;

use axum::{
    Json, Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::Serialize;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::errors::header_error::HeaderError;
use crate::webhooks::envelope::{WorkflowJobEnvelope, WorkflowRunEnvelope};
use crate::webhooks::event::WebhookEvent;

const MAX_BODY: usize = 25 * 1024 * 1024;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

pub trait Ingester {
    fn ingest(&self) -> Result<(), Box<dyn Error>>;
}

pub trait ParseHeader: Sized {
    fn parse_header(value: &str) -> Option<Self>;
}

pub trait FromHeader: ParseHeader {
    const HEADER: &'static str;
}

pub trait RequireHeader {
    fn require<T: FromHeader>(&self) -> Result<T, HeaderError>;
    fn require_named<T: ParseHeader>(&self, name: &'static str) -> Result<T, HeaderError>;
}

impl RequireHeader for HeaderMap {
    fn require<T: FromHeader>(&self) -> Result<T, HeaderError> {
        self.require_named(T::HEADER)
    }

    fn require_named<T: ParseHeader>(&self, name: &'static str) -> Result<T, HeaderError> {
        self.get(name)
            .ok_or(HeaderError::Missing(name))?
            .to_str()
            .ok()
            .and_then(T::parse_header)
            .ok_or(HeaderError::Invalid(name))
    }
}

impl ParseHeader for String {
    fn parse_header(value: &str) -> Option<Self> {
        Some(value.to_owned())
    }
}

async fn github_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let event: WebhookEvent = headers.require().map_err(|e| e.to_http())?;
    let delivery: String = headers
        .require_named("x-github-delivery")
        .map_err(|e| e.to_http())?;

    tracing::info!(
        event = event.as_str(),
        delivery,
        bytes = body.len(),
        "webhook received"
    );

    // A malformed payload is our bug, not GitHub's; log it and still ack, so
    // GitHub does not retry a delivery that will fail the same way again.
    if let Err(error) = process_webhook(event, body).await {
        tracing::error!(delivery, %error, "failed to process webhook");
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn process_webhook(event: WebhookEvent, body: Bytes) -> Result<(), Box<dyn Error>> {
    match event {
        WebhookEvent::WorkflowRun => serde_json::from_slice::<WorkflowRunEnvelope>(&body)?.ingest(),
        WebhookEvent::WorkflowJob => serde_json::from_slice::<WorkflowJobEnvelope>(&body)?.ingest(),
        _ => Ok(()),
    }
}

fn app() -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/github", post(github_webhook))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app()).await?;

    Ok(())
}
