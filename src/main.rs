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

mod webhooks;

const MAX_BODY: usize = 25 * 1024 * 1024;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

fn required_header<'a>(
    headers: &'a HeaderMap,
    name: &str,
) -> Result<&'a str, (StatusCode, String)> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("missing or invalid {name}"),
            )
        })
}

async fn github_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let event = required_header(&headers, "x-github-event")?;
    let delivery = required_header(&headers, "x-github-delivery")?;

    tracing::info!(event, delivery, bytes = body.len(), "webhook received");

    Ok(StatusCode::NO_CONTENT)
}

async fn process_webhook(event: String, delivery: String, body: String) {}

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
