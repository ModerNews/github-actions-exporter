use axum::{routing::get, Json, Router};
use serde::Serialize;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let app = Router::new()
        .route("/healthz", get(health))
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
