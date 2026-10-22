use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use serde::Serialize;

use crate::errors::header_error::HeaderError;
use crate::http::AppState;
use crate::http::headers::RequireHeader;
use crate::ingest::{Disposition, process_webhook};
use crate::metrics::github_actions_exporter_deliveries_total::{
    DeliveryOutcome, ExporterDeliveriesTotalLabels,
};
use crate::signature::Signature;
use crate::webhooks::event::WebhookEvent;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

type Response = Result<StatusCode, (StatusCode, String)>;

pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (labels, response) = deliver(&state, headers, body).await;

    state
        .metrics
        .exporter_deliveries_total
        .get_or_create(&labels)
        .inc();

    response
}

async fn deliver(
    state: &AppState,
    headers: HeaderMap,
    body: Bytes,
) -> (ExporterDeliveriesTotalLabels, Response) {
    let rejected_header = |event: &str, error: HeaderError| {
        (
            ExporterDeliveriesTotalLabels::new(event, "", DeliveryOutcome::RejectedHeader),
            Err(error.to_http()),
        )
    };

    let event: WebhookEvent = match headers.require() {
        Ok(event) => event,
        Err(error) => return rejected_header("", error),
    };

    let delivery: String = match headers.require_named("x-github-delivery") {
        Ok(delivery) => delivery,
        Err(error) => return rejected_header(event.as_str(), error),
    };

    let sig: Signature = match headers.require() {
        Ok(sig) => sig,
        Err(error) => return rejected_header(event.as_str(), error),
    };

    if state.verifier.verify(&sig, &body).is_err() {
        return (
            ExporterDeliveriesTotalLabels::new(
                event.as_str(),
                "",
                DeliveryOutcome::RejectedSignature,
            ),
            Err((StatusCode::UNAUTHORIZED, "invalid signature".to_owned())),
        );
    }

    tracing::info!(
        event = event.as_str(),
        delivery,
        bytes = body.len(),
        "webhook received"
    );

    let outcome = process_webhook(event.clone(), &body, &state.entity_state, &state.metrics).await;

    let (action, outcome) = match outcome {
        Ok(Disposition::Processed { action }) => (action, DeliveryOutcome::Processed),
        Ok(Disposition::Ignored) => (String::new(), DeliveryOutcome::Ignored),
        Err(error) => {
            tracing::error!(delivery, %error, "failed to process webhook");
            (String::new(), DeliveryOutcome::ParseError)
        }
    };

    (
        ExporterDeliveriesTotalLabels::new(event.as_str(), &action, outcome),
        Ok(StatusCode::NO_CONTENT),
    )
}

pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    Ok(state
        .scraper
        .render(
            &state.entity_state,
            &state.metrics,
            Utc::now(),
            state.entity_ttl,
        )
        .map_or(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render the metrics".into(),
            ),
            |v| (StatusCode::OK, v),
        ))
}
