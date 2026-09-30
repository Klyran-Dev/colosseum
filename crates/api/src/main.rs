use api::{
    AppState, JurisdictionProviderRegistry, JwtAuthenticator, TargetedRecoveryPorts, router,
};
use indexer::{MAX_RPC_RESPONSE_BYTES, ReqwestJsonRpcTransport};
use indexer_service::{MAX_TARGETED_RECOVERY_PAGES, build_targeted_recovery_sources};
use observability::ServiceMetrics;
use settlement_coordinator::{DEFAULT_ATTEMPT_TIMEOUT, SettlementCoordinator};
use std::sync::Arc;
use std::{env, net::SocketAddr};
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
    let storage = match env::var("API_DATABASE_URL") {
        Ok(database_url) => Some(Arc::new(PgStore::from_url_with_evidence_cipher(
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
        Err(_) => None,
    };
    if let Some(store) = &storage {
        store.ping().await?;
    }
    let issuer = env::var("API_AUTH_ISSUER").ok();
    let audience = env::var("API_AUTH_AUDIENCE").ok();
    let jwks_json = match env::var("API_AUTH_JWKS_JSON") {
        Ok(jwks_json) => Some(jwks_json),
        Err(_) => {
            match (
                env::var("API_AUTH_JWKS_KID"),
                env::var("API_AUTH_JWKS_PUBLIC_KEY"),
            ) {
                (Ok(kid), Ok(public_key)) => Some(
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
                (Err(_), Err(_)) => None,
                _ => {
                    return Err("API_AUTH_JWKS_KID and API_AUTH_JWKS_PUBLIC_KEY must be configured together".into());
                }
            }
        }
    };
    let authenticator = match (issuer, audience, jwks_json) {
        (Some(issuer), Some(audience), Some(jwks_json)) => Some(JwtAuthenticator::from_jwks_json(
            issuer, audience, &jwks_json,
        )?),
        (None, None, None) => None,
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
    axum::serve(listener, router().with_state(state)).await?;
    telemetry.shutdown();
    Ok(())
}
