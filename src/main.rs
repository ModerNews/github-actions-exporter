use anyhow::Result;
use tracing_subscriber::EnvFilter;

use github_actions_exporter::config::Config;
use github_actions_exporter::http::{AppState, app};
use github_actions_exporter::signature::Verifier;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(Verifier::new(&config.webhook_secret)?);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app(state)).await?;

    Ok(())
}
