//! Process configuration, read from the environment once at startup.

use std::net::SocketAddr;

use anyhow::Context;

const GITHUB_WEBHOOK_SECRET_KEY: &str = "GITHUB_WEBHOOK_SECRET";
const BIND_ADDR_KEY: &str = "BIND_ADDR";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";

pub struct Config {
    pub bind_addr: SocketAddr,
    pub webhook_secret: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let webhook_secret = std::env::var(GITHUB_WEBHOOK_SECRET_KEY)
            .with_context(|| format!("{GITHUB_WEBHOOK_SECRET_KEY} must be set"))?;
        anyhow::ensure!(
            !webhook_secret.is_empty(),
            "{GITHUB_WEBHOOK_SECRET_KEY} must not be empty"
        );

        let bind_addr = std::env::var(BIND_ADDR_KEY)
            .unwrap_or_else(|_| DEFAULT_BIND_ADDR.into())
            .parse()
            .with_context(|| format!("{BIND_ADDR_KEY} must be host:port"))?;

        Ok(Self {
            bind_addr,
            webhook_secret,
        })
    }
}
