#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use std::{env, net::SocketAddr, sync::mpsc};

use tax_engine_service::{
    DEFAULT_CACHE_CAPACITY, DEFAULT_MAX_CONCURRENCY, EngineService, MAX_CACHE_CAPACITY,
    MAX_CONCURRENCY,
};
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Config {
    environment: Environment,
    bind_address: SocketAddr,
    auth_token: String,
    cache_capacity: usize,
    max_concurrency: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Environment {
    Local,
    Staging,
    Production,
}

impl Environment {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            _ => Err("TAX_ENGINE_ENV must be local, staging, or production".to_string()),
        }
    }
}

impl Config {
    fn from_values(
        environment: &str,
        bind_address: &str,
        auth_token: &str,
        cache_capacity: &str,
        max_concurrency: &str,
    ) -> Result<Self, String> {
        let environment = Environment::parse(environment)?;
        let auth_token = auth_token.trim();
        if auth_token.is_empty() || auth_token.len() > 256 {
            return Err("TAX_ENGINE_AUTH_TOKEN must contain 1 to 256 characters".to_string());
        }
        if environment != Environment::Local && auth_token == "local-development-only" {
            return Err(
                "TAX_ENGINE_AUTH_TOKEN cannot use the local development token outside local"
                    .to_string(),
            );
        }
        let cache_capacity = cache_capacity
            .parse::<usize>()
            .map_err(|_| "TAX_ENGINE_CACHE_CAPACITY must be a positive integer".to_string())?;
        if !(1..=MAX_CACHE_CAPACITY).contains(&cache_capacity) {
            return Err(format!(
                "TAX_ENGINE_CACHE_CAPACITY must be between 1 and {MAX_CACHE_CAPACITY}"
            ));
        }
        let max_concurrency = max_concurrency
            .parse::<usize>()
            .map_err(|_| "TAX_ENGINE_MAX_CONCURRENCY must be a positive integer".to_string())?;
        if !(1..=MAX_CONCURRENCY).contains(&max_concurrency) {
            return Err(format!(
                "TAX_ENGINE_MAX_CONCURRENCY must be between 1 and {MAX_CONCURRENCY}"
            ));
        }
        Ok(Self {
            environment,
            bind_address: bind_address.parse().map_err(|_| {
                "TAX_ENGINE_BIND_ADDRESS must be a valid socket address".to_string()
            })?,
            auth_token: auth_token.to_string(),
            cache_capacity,
            max_concurrency,
        })
    }

    fn from_env() -> Result<Self, String> {
        Self::from_values(
            &required_env("TAX_ENGINE_ENV")?,
            &env::var("TAX_ENGINE_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8084".to_string()),
            &required_env("TAX_ENGINE_AUTH_TOKEN")?,
            &env::var("TAX_ENGINE_CACHE_CAPACITY")
                .unwrap_or_else(|_| DEFAULT_CACHE_CAPACITY.to_string()),
            &env::var("TAX_ENGINE_MAX_CONCURRENCY")
                .unwrap_or_else(|_| DEFAULT_MAX_CONCURRENCY.to_string()),
        )
    }
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name)
        .map_err(|_| {
            format!("{name} is required; refusing to start without explicit configuration")
        })
        .and_then(|value| validate_required(name, &value))
}

fn validate_required(name: &str, value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(value.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut telemetry = observability::init_tracing("tax-engine")?;
    let config = Config::from_env()?;
    let service = EngineService::with_limits(config.cache_capacity, config.max_concurrency)?;
    let listener = TcpListener::bind(config.bind_address).await?;
    info!(
        address = %config.bind_address,
        environment = ?config.environment,
        cache_capacity = config.cache_capacity,
        max_concurrency = config.max_concurrency,
        "tax engine service listening"
    );

    let (shutdown_sender, shutdown_receiver) = mpsc::sync_channel(1);
    let shutdown = async move {
        let result = tokio::signal::ctrl_c()
            .await
            .map_err(|error| error.to_string());
        if shutdown_sender.send(result).is_err() {
            tracing::warn!("tax engine shutdown result could not be recorded");
        }
    };
    axum::serve(listener, service.router(&config.auth_token))
        .with_graceful_shutdown(shutdown)
        .await?;
    match shutdown_receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Err(format!("tax engine shutdown signal listener failed: {error}").into());
        }
        Err(_) => {
            return Err(
                "tax engine server ended before a shutdown signal result was recorded".into(),
            );
        }
    }
    telemetry.shutdown();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Config, Environment, validate_required};

    #[test]
    fn environment_is_explicit_and_bounded() {
        assert_eq!(Environment::parse("local"), Ok(Environment::Local));
        assert_eq!(
            Environment::parse("production"),
            Ok(Environment::Production)
        );
        assert!(Environment::parse("development").is_err());
    }

    #[test]
    fn configuration_rejects_missing_or_unsafe_values() {
        assert!(Config::from_values("local", "0.0.0.0:8084", "", "64", "32").is_err());
        assert!(Config::from_values("local", "not-an-address", "token", "64", "32").is_err());
        assert!(Config::from_values("local", "0.0.0.0:8084", "token", "0", "32").is_err());
        assert!(Config::from_values("local", "0.0.0.0:8084", "token", "64", "0").is_err());
        assert!(
            Config::from_values(
                "staging",
                "0.0.0.0:8084",
                "local-development-only",
                "64",
                "32"
            )
            .is_err()
        );
    }

    #[test]
    fn required_environment_rejects_empty_values() {
        assert!(validate_required("TAX_ENGINE_ENV", " ").is_err());
        assert_eq!(
            validate_required("TAX_ENGINE_ENV", "local"),
            Ok("local".to_string())
        );
    }
}
