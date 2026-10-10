mod errors;
mod webhooks;

use std::error::Error;
use std::net::SocketAddr;

use anyhow::Context;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha2::Sha256;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::errors::{header_error::HeaderError, signature_error::SignatureError};
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

pub struct Signature([u8; 32]);

type HmacSha256 = Hmac<Sha256>;

impl Signature {
    pub fn verify(&self, mut mac: HmacSha256, body: &[u8]) -> Result<(), SignatureError> {
        mac.update(body);
        mac.verify_slice(&self.0)
            .map_err(|_| SignatureError::Invalid)
    }
}

impl FromHeader for Signature {
    const HEADER: &'static str = "x-hub-signature-256";
}

impl ParseHeader for Signature {
    fn parse_header(value: &str) -> Option<Self> {
        let stripped = value.strip_prefix("sha256=")?;
        let mut tag = [0u8; 32];
        hex::decode_to_slice(stripped, &mut tag).ok()?;
        Some(Self(tag))
    }
}

async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let event: WebhookEvent = headers.require().map_err(|e| e.to_http())?;
    let delivery: String = headers
        .require_named("x-github-delivery")
        .map_err(|e| e.to_http())?;
    let sig: Signature = headers.require().map_err(|e| e.to_http())?;

    sig.verify(state.verifier, &body)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid signature".to_owned()))?;

    tracing::info!(
        event = event.as_str(),
        delivery,
        bytes = body.len(),
        "webhook received"
    );

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

const GITHUB_WEBHOOK_SECRET_KEY: &str = "GITHUB_WEBHOOK_SECRET";
const BIND_ADDR_KEY: &str = "BIND_ADDR";

#[derive(Clone)]
struct AppState {
    verifier: HmacSha256,
}

impl AppState {
    fn from_env() -> anyhow::Result<Self> {
        let secret = std::env::var(GITHUB_WEBHOOK_SECRET_KEY)
            .with_context(|| format!("{GITHUB_WEBHOOK_SECRET_KEY} must be set"))?;
        anyhow::ensure!(
            !secret.is_empty(),
            "{GITHUB_WEBHOOK_SECRET_KEY} must not be empty"
        );
        Ok(Self {
            verifier: HmacSha256::new_from_slice(secret.as_bytes())?,
        })
    }
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/github", post(github_webhook))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let state = AppState::from_env()?;

    let addr: SocketAddr = std::env::var(BIND_ADDR_KEY)
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()
        .context(format!("{BIND_ADDR_KEY} must be host:port"))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("listening on {addr}");
    axum::serve(listener, app(state)).await?;

    Ok(())
}
