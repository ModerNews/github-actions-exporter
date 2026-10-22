use anyhow::Result;

use github_actions_exporter::config::Config;
use github_actions_exporter::http::{AppState, app};
use github_actions_exporter::signature::Verifier;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(config.log_filter)
        .init();

    let state = AppState::new(Verifier::new(&config.webhook_secret)?, config.entity_ttl);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;

    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app(state)).await?;

    Ok(())
}
