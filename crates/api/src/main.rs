#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use api::{
    AppState, JurisdictionProviderRegistry, JwtAuthenticator, TargetedRecoveryPorts, router,
};
use indexer::{MAX_RPC_RESPONSE_BYTES, ReqwestJsonRpcTransport};
use indexer_service::{MAX_TARGETED_RECOVERY_PAGES, build_targeted_recovery_sources};
use observability::ServiceMetrics;
use settlement_coordinator::{DEFAULT_ATTEMPT_TIMEOUT, SettlementCoordinator};
use std::sync::{Arc, mpsc};
use std::{env, net::SocketAddr, path::PathBuf};
use storage::{EvidenceCipher, PgStore};
use tokio::net::TcpListener;
use tracing::info;
mod config;
use config::DeploymentConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut telemetry = observability::init_tracing("api")?;

    let bind_address = env::var("API_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let address: SocketAddr = bind_address.parse()?;
    let deployment = DeploymentConfig::from_env()?;
    let env_value = |name: &str| env::var(name).ok().filter(|value| !value.trim().is_empty());
    let evidence_cipher = match (
        env_value("API_EVIDENCE_KEY_RING_JSON"),
        env_value("API_EVIDENCE_KEY_ID"),
        env_value("API_EVIDENCE_KEY_B64"),
    ) {
        (Some(document), None, None) => Some(EvidenceCipher::from_key_ring_json(&document)?),
        (Some(_), _, _) => {
            return Err(
                "API_EVIDENCE_KEY_RING_JSON cannot be combined with legacy evidence-key variables"
                    .into(),
            );
        }
        (None, Some(key_id), Some(encoded_key)) => {
            Some(EvidenceCipher::from_base64(key_id, &encoded_key)?)
        }
        (None, None, None) => None,
        (None, _, _) => {
            return Err(
                "API_EVIDENCE_KEY_ID and API_EVIDENCE_KEY_B64 must be configured together".into(),
            );
        }
    };
    let storage = match env_value("API_DATABASE_URL") {
        Some(database_url) => Some(Arc::new(PgStore::from_url_with_evidence_cipher(
            &database_url,
            env::var("API_DATABASE_POOL_SIZE")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(storage::DEFAULT_POOL_SIZE),
            storage::DEFAULT_POOL_WAIT_TIMEOUT,
            tax_engine::ENGINE_VERSION,
            Some(evidence_cipher.clone().ok_or(
                "an evidence key ring or API_EVIDENCE_KEY_ID/API_EVIDENCE_KEY_B64 is required when API_DATABASE_URL is configured",
            )?),
        )?)),
        None => None,
    };
    if let Some(store) = &storage {
        store.ping().await?;
    }
    let issuer = env::var("API_AUTH_ISSUER").ok();
    let audience = env::var("API_AUTH_AUDIENCE").ok();
    let jwks_file = match env::var("API_AUTH_JWKS_FILE") {
        Ok(path) if path.trim().is_empty() => {
            return Err("API_AUTH_JWKS_FILE cannot be empty".into());
        }
        Ok(path) => Some(PathBuf::from(path)),
        Err(_) => None,
    };
    let jwks_json = match (
        env_value("API_AUTH_JWKS_JSON"),
        env_value("API_AUTH_JWKS_KID"),
        env_value("API_AUTH_JWKS_PUBLIC_KEY"),
    ) {
        (Some(jwks_json), None, None) => Some(jwks_json),
        (None, Some(kid), Some(public_key)) => Some(
            serde_json::json!({
                "keys": [{
                    "kid": kid,
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "alg": "EdDSA",
                    "x": public_key
                }]
            })
            .to_string(),
        ),
        (None, None, None) => None,
        _ => {
            return Err("API_AUTH_JWKS_JSON or the API_AUTH_JWKS_KID/API_AUTH_JWKS_PUBLIC_KEY pair must be configured exclusively".into());
        }
    };
    if jwks_file.is_some()
        && (env_value("API_AUTH_JWKS_JSON").is_some()
            || env_value("API_AUTH_JWKS_KID").is_some()
            || env_value("API_AUTH_JWKS_PUBLIC_KEY").is_some())
    {
        return Err(
            "API_AUTH_JWKS_FILE cannot be combined with inline authentication key configuration"
                .into(),
        );
    }
    let (authenticator, auth_jwks_file) = match (issuer, audience, jwks_file, jwks_json) {
        (Some(issuer), Some(audience), Some(path), None) => (
            Some(JwtAuthenticator::from_jwks_file(issuer, audience, &path)?),
            Some(path),
        ),
        (Some(issuer), Some(audience), None, Some(jwks_json)) => (
            Some(JwtAuthenticator::from_jwks_json(
                issuer, audience, &jwks_json,
            )?),
            None,
        ),
        (None, None, None, None) => (None, None),
        _ => {
            return Err("API_AUTH_ISSUER, API_AUTH_AUDIENCE, and authentication keys must be configured together".into());
        }
    };
    let jurisdiction_provider_registry = match env::var("API_JURISDICTION_PROVIDER_KEYS_JSON") {
        Ok(document) => Some(Arc::new(JurisdictionProviderRegistry::from_json(
            &document,
        )?)),
        Err(_) => match (
            env::var("API_JURISDICTION_PROVIDER_KEY_ID"),
            env::var("API_JURISDICTION_PROVIDER_PUBLIC_KEY"),
        ) {
            (Ok(key_id), Ok(public_key)) => {
                let document = serde_json::json!({
                    "keys": [{"key_id": key_id, "public_key": public_key}]
                });
                Some(Arc::new(JurisdictionProviderRegistry::from_json(
                    &document.to_string(),
                )?))
            }
            (Err(_), Err(_)) => None,
            _ => {
                return Err("API_JURISDICTION_PROVIDER_KEY_ID and API_JURISDICTION_PROVIDER_PUBLIC_KEY must be configured together".into());
            }
        },
    };
    let simulation_mode = deployment.simulation_mode;
    if !simulation_mode && jurisdiction_provider_registry.is_none() {
        return Err(
            "jurisdiction provider verification is required outside simulation mode".into(),
        );
    }
    let settlement_coordinator = match env::var("API_SETTLEMENT_RPC_ENDPOINTS")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => {
            let endpoints = value
                .split(',')
                .map(str::trim)
                .filter(|endpoint| !endpoint.is_empty())
                .collect::<Vec<_>>();
            if endpoints.is_empty() || endpoints.len() > 3 {
                return Err("API_SETTLEMENT_RPC_ENDPOINTS must contain 1 to 3 endpoints".into());
            }
            let local_development = deployment.allows_insecure_local_rpc();
            let mut providers = Vec::with_capacity(endpoints.len());
            for endpoint in endpoints {
                let provider = if local_development {
                    ReqwestJsonRpcTransport::new_for_local_development(
                        endpoint,
                        DEFAULT_ATTEMPT_TIMEOUT,
                        MAX_RPC_RESPONSE_BYTES,
                    )?
                } else {
                    ReqwestJsonRpcTransport::new(
                        endpoint,
                        DEFAULT_ATTEMPT_TIMEOUT,
                        MAX_RPC_RESPONSE_BYTES,
                    )?
                };
                providers.push(Arc::new(provider) as Arc<dyn indexer::JsonRpcTransport>);
            }
            Some(Arc::new(SettlementCoordinator::new(
                providers,
                DEFAULT_ATTEMPT_TIMEOUT,
            )?))
        }
        None => None,
    };
    let targeted_recovery = match env_value("API_INDEXER_RPC_ENDPOINTS") {
        Some(value) => {
            let endpoints = value
                .split(',')
                .map(str::trim)
                .filter(|endpoint| !endpoint.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            let stream_key = env_value("API_INDEXER_STREAM_KEY").ok_or(
                "API_INDEXER_STREAM_KEY is required when targeted recovery RPC is configured",
            )?;
            let max_pages = env_value("API_INDEXER_TARGETED_RECOVERY_MAX_PAGES")
                .map(|value| value.parse::<usize>())
                .transpose()?
                .unwrap_or(MAX_TARGETED_RECOVERY_PAGES);
            if max_pages == 0 || max_pages > MAX_TARGETED_RECOVERY_PAGES {
                return Err(
                    "API_INDEXER_TARGETED_RECOVERY_MAX_PAGES is outside the bounded range".into(),
                );
            }
            let local_development = deployment.allows_insecure_local_rpc();
            let (signature_source, transaction_source) = build_targeted_recovery_sources(
                &endpoints,
                DEFAULT_ATTEMPT_TIMEOUT,
                local_development,
            )?;
            Some(Arc::new(TargetedRecoveryPorts {
                signature_source: Arc::new(signature_source),
                transaction_source: Arc::new(transaction_source),
                stream_key,
                max_pages,
            }))
        }
        None => None,
    };
    let shutdown_authenticator = authenticator.clone();
    let shutdown_jwks_file = auth_jwks_file.clone();
    let state = AppState {
        ready: storage.is_some()
            && authenticator.is_some()
            && (simulation_mode || jurisdiction_provider_registry.is_some())
            && env::var("API_READY").as_deref() != Ok("false"),
        simulation_mode,
        storage,
        authenticator,
        jurisdiction_provider_registry,
        settlement_coordinator,
        targeted_recovery,
        metrics: std::sync::Arc::new(ServiceMetrics::new(
            "api",
            &[
                "tax_calculate",
                "tax_document_calculate",
                "tax_quote",
                "tax_replay",
                "tax_rule_transition",
                "tax_rule_governance_read",
                "vault_release_claim_create",
                "vault_release_claim_read",
                "vault_release_claim_transition",
                "adjustment_authority_challenge",
                "adjustment_authority_bind",
                "settlement_adjustment_compile",
                "vault_reconciliation_read",
                "indexer_replay_operator",
                "indexer_targeted_recovery",
                "settlement_compile",
                "settlement_submit",
            ],
        )?),
    };
    let listener = TcpListener::bind(address).await?;
    info!(%address, "api listening");
    let (shutdown_sender, shutdown_receiver) = mpsc::sync_channel(1);
    let shutdown = async move {
        let result = wait_for_shutdown(shutdown_authenticator, shutdown_jwks_file).await;
        if shutdown_sender.send(result).is_err() {
            tracing::warn!("api shutdown result could not be recorded");
        }
    };
    axum::serve(listener, router().with_state(state))
        .with_graceful_shutdown(shutdown)
        .await?;
    match shutdown_receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Err(format!("api shutdown signal listener failed: {error}").into());
        }
        Err(_) => {
            return Err("api server ended before a shutdown signal result was recorded".into());
        }
    }
    telemetry.shutdown();
    Ok(())
}

async fn wait_for_shutdown(
    authenticator: Option<JwtAuthenticator>,
    jwks_file: Option<PathBuf>,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        let mut hangup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
            .map_err(|error| format!("api SIGHUP listener failed: {error}"))?;
        loop {
            tokio::select! {
                result = tokio::signal::ctrl_c() => {
                    return result.map_err(|error| format!("api shutdown signal listener failed: {error}"));
                }
                signal = hangup.recv() => {
                    if signal.is_none() {
                        return Err("api SIGHUP listener ended unexpectedly".into());
                    }
                    let (Some(authenticator), Some(path)) =
                        (authenticator.clone(), jwks_file.clone()) else {
                        tracing::warn!("api received SIGHUP without a file-backed JWKS configuration");
                        continue;
                    };
                    let reload = tokio::task::spawn_blocking(move || {
                        authenticator.reload_from_jwks_file(&path)
                    })
                    .await
                    .map_err(|error| format!("api JWKS reload task failed: {error}"))?;
                    match reload {
                        Ok(key_count) => {
                            tracing::info!(key_count, "api authentication JWKS reloaded");
                        }
                        Err(error) => {
                            tracing::warn!(error = %error, "api authentication JWKS reload rejected; retaining last known-good snapshot");
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (authenticator, jwks_file);
        tokio::signal::ctrl_c()
            .await
            .map_err(|error| format!("api shutdown signal listener failed: {error}"))
    }
}
