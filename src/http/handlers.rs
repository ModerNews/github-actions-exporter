use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;

use crate::http::AppState;
use crate::http::headers::RequireHeader;
use crate::signature::Signature;
use crate::webhooks::event::WebhookEvent;
use crate::webhooks::process_webhook;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let event: WebhookEvent = headers.require().map_err(|e| e.to_http())?;
    let delivery: String = headers
        .require_named("x-github-delivery")
        .map_err(|e| e.to_http())?;
    let sig: Signature = headers.require().map_err(|e| e.to_http())?;

    state
        .verifier
        .verify(&sig, &body)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid signature".to_owned()))?;

    tracing::info!(
        event = event.as_str(),
        delivery,
        bytes = body.len(),
        "webhook received"
    );

    if let Err(error) = process_webhook(event, &body, &state.metrics).await {
        tracing::error!(delivery, %error, "failed to process webhook");
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    Ok(state.metrics.encode().map_or(
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to render the metrics".into(),
        ),
        |v| (StatusCode::OK, v),
    ))
}
