//! Process configuration, read from the environment once at startup.

use std::net::SocketAddr;

use anyhow::Context;
use chrono::TimeDelta;
use tracing_subscriber::EnvFilter;

const GITHUB_WEBHOOK_SECRET_KEY: &str = "GITHUB_WEBHOOK_SECRET";
const BIND_ADDR_KEY: &str = "BIND_ADDR";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";
const ENTITY_TTL_HOURS_KEY: &str = "ENTITY_TTL_HOURS";
const DEFAULT_ENTITY_TTL_HOURS: i64 = 24;
const LOG_FILTER_KEY: &str = "RUST_LOG";
const DEFAULT_LOG_FILTER: &str = "info,tower_http=debug";

pub struct Config {
    pub bind_addr: SocketAddr,
    pub webhook_secret: String,
    pub entity_ttl: TimeDelta,
    pub log_filter: EnvFilter,
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

        let entity_ttl_hours = match std::env::var(ENTITY_TTL_HOURS_KEY) {
            Ok(value) => value.parse().with_context(|| {
                format!("{ENTITY_TTL_HOURS_KEY} must be a whole number of hours")
            })?,
            Err(_) => DEFAULT_ENTITY_TTL_HOURS,
        };
        let entity_ttl = TimeDelta::try_hours(entity_ttl_hours)
            .with_context(|| format!("{ENTITY_TTL_HOURS_KEY} is out of range"))?;
        anyhow::ensure!(
            entity_ttl > TimeDelta::zero(),
            "{ENTITY_TTL_HOURS_KEY} must be positive; a zero TTL evicts every entity on the next sweep"
        );

        let log_filter =
            std::env::var(LOG_FILTER_KEY).unwrap_or_else(|_| DEFAULT_LOG_FILTER.to_owned());
        let log_filter = EnvFilter::try_new(&log_filter)
            .with_context(|| format!("{LOG_FILTER_KEY} is not a valid filter directive"))?;

        Ok(Self {
            bind_addr,
            webhook_secret,
            entity_ttl,
            log_filter,
        })
    }
}
