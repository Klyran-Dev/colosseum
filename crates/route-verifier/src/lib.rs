use indexer::{
    JsonRpcTransport, MAX_PROVIDER_ATTEMPTS, MAX_ROUTE_RPC_RESULT_BYTES,
    RefundFundingTopologyConfig, RefundFundingTopologyVerification, RefundFundingVerificationError,
    ReqwestJsonRpcTransport, RouteVerificationError, SettlementRouteConfig,
    SettlementRouteVerification, verify_refund_funding_topology, verify_settlement_route,
};
use std::{env, sync::Arc, time::Duration};
use storage::{
    DEFAULT_POOL_WAIT_TIMEOUT, PgStore, SettlementRouteVerificationCandidate, StorageError,
};
use thiserror::Error;
use tokio::time::timeout;

pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(2);
pub const DEFAULT_BATCH_SIZE: usize = 8;
pub const DEFAULT_POOL_SIZE: usize = 8;
pub const MAX_BATCH_SIZE: usize = 64;
const REFUND_FUNDING_VERIFIER_ACTOR: &str = "system:refund-funding-verifier";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentEnvironment {
    Local,
    Staging,
    Production,
}

impl DeploymentEnvironment {
    fn allows_insecure_rpc(self, enabled: bool) -> bool {
        !enabled || matches!(self, Self::Local)
    }
}

fn parse_deployment_environment(value: &str) -> Result<DeploymentEnvironment, RouteVerifierError> {
    match value {
        "local" => Ok(DeploymentEnvironment::Local),
        "staging" => Ok(DeploymentEnvironment::Staging),
        "production" => Ok(DeploymentEnvironment::Production),
        _ => Err(RouteVerifierError::Configuration(
            "ROUTE_VERIFIER_ENV must be exactly local, staging, or production".to_string(),
        )),
    }
}

#[derive(Clone, Debug)]
pub struct RouteVerifierConfig {
    pub environment: DeploymentEnvironment,
    pub database_url: String,
    pub rpc_endpoints: Vec<String>,
    pub poll_interval: Duration,
    pub rpc_timeout: Duration,
    pub database_pool_size: usize,
    pub batch_size: usize,
    pub allow_insecure_local_rpc: bool,
}

impl RouteVerifierConfig {
    pub fn from_env() -> Result<Self, RouteVerifierError> {
        let environment = parse_deployment_environment(&required_env("ROUTE_VERIFIER_ENV")?)?;
        let database_url = required_env("ROUTE_VERIFIER_DATABASE_URL")?;
        let rpc_endpoints = required_env("ROUTE_VERIFIER_RPC_ENDPOINTS")?
            .split(',')
            .map(str::trim)
            .filter(|endpoint| !endpoint.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if rpc_endpoints.is_empty() || rpc_endpoints.len() > MAX_PROVIDER_ATTEMPTS {
            return Err(RouteVerifierError::Configuration(
                "ROUTE_VERIFIER_RPC_ENDPOINTS must contain between 1 and 3 endpoints".to_string(),
            ));
        }
        let poll_interval = parse_duration_ms(
            "ROUTE_VERIFIER_POLL_MS",
            DEFAULT_POLL_INTERVAL,
            100..=300_000,
        )?;
        let rpc_timeout = parse_duration_ms(
            "ROUTE_VERIFIER_RPC_TIMEOUT_MS",
            DEFAULT_RPC_TIMEOUT,
            50..=10_000,
        )?;
        let database_pool_size =
            parse_usize("ROUTE_VERIFIER_DATABASE_POOL_SIZE", DEFAULT_POOL_SIZE)?;
        if database_pool_size == 0 || database_pool_size > 64 {
            return Err(RouteVerifierError::Configuration(
                "ROUTE_VERIFIER_DATABASE_POOL_SIZE must be between 1 and 64".to_string(),
            ));
        }
        let batch_size = parse_usize("ROUTE_VERIFIER_BATCH_SIZE", DEFAULT_BATCH_SIZE)?;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(RouteVerifierError::Configuration(format!(
                "ROUTE_VERIFIER_BATCH_SIZE must be between 1 and {MAX_BATCH_SIZE}"
            )));
        }
        let allow_insecure_local_rpc =
            parse_bool("ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC", false)?;
        if !environment.allows_insecure_rpc(allow_insecure_local_rpc) {
            return Err(RouteVerifierError::Configuration(
                "ROUTE_VERIFIER_ALLOW_INSECURE_LOCAL_RPC requires ROUTE_VERIFIER_ENV=local"
                    .to_string(),
            ));
        }
        Ok(Self {
            environment,
            database_url,
            rpc_endpoints,
            poll_interval,
            rpc_timeout,
            database_pool_size,
            batch_size,
            allow_insecure_local_rpc,
        })
    }

    pub fn build_transports(&self) -> Result<Vec<Arc<dyn JsonRpcTransport>>, RouteVerifierError> {
        self.rpc_endpoints
            .iter()
            .map(|endpoint| {
                let transport = if self.allow_insecure_local_rpc {
                    ReqwestJsonRpcTransport::new_for_local_development(
                        endpoint,
                        self.rpc_timeout,
                        MAX_ROUTE_RPC_RESULT_BYTES,
                    )
                    .map_err(|error| RouteVerifierError::Configuration(error.to_string()))?
                } else {
                    ReqwestJsonRpcTransport::new(
                        endpoint,
                        self.rpc_timeout,
                        MAX_ROUTE_RPC_RESULT_BYTES,
                    )
                    .map_err(|error| RouteVerifierError::Configuration(error.to_string()))?
                };
                Ok(Arc::new(transport) as Arc<dyn JsonRpcTransport>)
            })
            .collect()
    }
}

#[derive(Debug, Error)]
pub enum RouteVerifierError {
    #[error("route verifier configuration is invalid: {0}")]
    Configuration(String),
    #[error("route verifier storage failed: {0}")]
    Storage(#[from] StorageError),
    #[error("route verifier account proof failed: {0}")]
    Verification(#[from] RouteVerificationError),
    #[error("refund funding verifier account proof failed: {0}")]
    RefundFundingVerification(#[from] RefundFundingVerificationError),
    #[error("all configured RPC providers failed: {0}")]
    ProvidersExhausted(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerificationSummary {
    pub examined: usize,
    pub activated: usize,
    pub already_active: usize,
    pub failed: usize,
}

pub async fn verify_once(
    store: &PgStore,
    transports: &[Arc<dyn JsonRpcTransport>],
    batch_size: usize,
    rpc_timeout: Duration,
) -> Result<VerificationSummary, RouteVerifierError> {
    if transports.is_empty() || transports.len() > MAX_PROVIDER_ATTEMPTS {
        return Err(RouteVerifierError::Configuration(
            "route verifier provider pool is outside the bounded range".to_string(),
        ));
    }
    if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
        return Err(RouteVerifierError::Configuration(
            "route verifier batch size is outside the bounded range".to_string(),
        ));
    }
    if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&rpc_timeout) {
        return Err(RouteVerifierError::Configuration(
            "route verifier RPC timeout is outside the bounded range".to_string(),
        ));
    }
    let candidates = store
        .list_settlement_routes_for_verification(batch_size)
        .await?;
    let mut summary = VerificationSummary {
        examined: candidates.len(),
        ..VerificationSummary::default()
    };
    for candidate in candidates {
        let route = route_config(&candidate);
        match verify_with_failover(transports, &route, rpc_timeout).await {
            Ok(verification) => match store
                .activate_settlement_route(candidate.merchant_id, candidate.id, &verification)
                .await
            {
                Ok(true) => summary.activated += 1,
                Ok(false) => summary.already_active += 1,
                Err(error) => {
                    summary.failed += 1;
                    tracing::warn!(route_id = %candidate.id, %error, "route verification evidence could not be activated");
                }
            },
            Err(error) => {
                summary.failed += 1;
                tracing::warn!(route_id = %candidate.id, %error, "route account verification failed closed");
            }
        }
    }
    Ok(summary)
}

pub async fn verify_refund_funding_once(
    store: &PgStore,
    transports: &[Arc<dyn JsonRpcTransport>],
    batch_size: usize,
    rpc_timeout: Duration,
) -> Result<VerificationSummary, RouteVerifierError> {
    validate_verifier_bounds(transports, batch_size, rpc_timeout)?;
    let candidates = store
        .list_refund_funding_configurations_for_verification(batch_size)
        .await?;
    let mut summary = VerificationSummary {
        examined: candidates.len(),
        ..VerificationSummary::default()
    };
    for candidate in candidates {
        let claim_token = match store
            .begin_refund_funding_verification(
                candidate.merchant_id,
                candidate.id,
                REFUND_FUNDING_VERIFIER_ACTOR,
            )
            .await
        {
            Ok(Some(claim_token)) => claim_token,
            Ok(None) => continue,
            Err(error) => {
                summary.failed += 1;
                tracing::warn!(configuration_id = %candidate.id, %error, "refund funding verification claim failed");
                continue;
            }
        };
        let topology = RefundFundingTopologyConfig {
            configuration_id: candidate.id,
            version: candidate.version,
            configuration_digest: candidate.configuration_digest,
            merchant_account: candidate.merchant_account,
            mint: candidate.mint,
            tax_vault: candidate.tax_vault,
            vault_authority: candidate.vault_authority,
            refund_funder: candidate.refund_funder,
            net_source: candidate.net_source,
            token_program: candidate.token_program,
        };
        match verify_refund_funding_with_failover(transports, &topology, rpc_timeout).await {
            Ok(verification) => match store
                .activate_refund_funding_configuration(
                    candidate.merchant_id,
                    candidate.id,
                    claim_token,
                    &verification,
                )
                .await
            {
                Ok(true) => summary.activated += 1,
                Ok(false) => summary.already_active += 1,
                Err(error) => {
                    summary.failed += 1;
                    let _ = store
                        .release_refund_funding_verification(
                            candidate.merchant_id,
                            candidate.id,
                            claim_token,
                        )
                        .await;
                    tracing::warn!(configuration_id = %candidate.id, %error, "refund funding verification evidence could not be activated");
                }
            },
            Err(error) => {
                summary.failed += 1;
                if let Err(release_error) = store
                    .release_refund_funding_verification(
                        candidate.merchant_id,
                        candidate.id,
                        claim_token,
                    )
                    .await
                {
                    tracing::warn!(configuration_id = %candidate.id, %release_error, "refund funding verification lease release failed");
                }
                tracing::warn!(configuration_id = %candidate.id, %error, "refund funding topology verification failed closed");
            }
        }
    }
    Ok(summary)
}

async fn verify_with_failover(
    transports: &[Arc<dyn JsonRpcTransport>],
    route: &SettlementRouteConfig,
    rpc_timeout: Duration,
) -> Result<SettlementRouteVerification, RouteVerifierError> {
    let mut successes = Vec::new();
    let mut last_error = None;
    for transport in transports {
        match timeout(
            rpc_timeout,
            verify_settlement_route(transport.as_ref(), route),
        )
        .await
        {
            Ok(Ok(verification)) => successes.push(verification),
            Ok(Err(error)) => last_error = Some(error.to_string()),
            Err(_) => last_error = Some("RPC verification deadline exceeded".to_string()),
        }
    }
    select_agreed_verification(successes, last_error)
}

fn select_agreed_verification(
    successes: Vec<SettlementRouteVerification>,
    last_error: Option<String>,
) -> Result<SettlementRouteVerification, RouteVerifierError> {
    let Some(first) = successes.first().cloned() else {
        return Err(RouteVerifierError::ProvidersExhausted(
            last_error.unwrap_or_else(|| "no provider returned a result".to_string()),
        ));
    };
    if successes
        .iter()
        .any(|candidate| candidate.verification_digest != first.verification_digest)
    {
        return Err(RouteVerifierError::ProvidersExhausted(
            "configured providers returned conflicting finalized route evidence".to_string(),
        ));
    }
    Ok(first)
}

async fn verify_refund_funding_with_failover(
    transports: &[Arc<dyn JsonRpcTransport>],
    topology: &RefundFundingTopologyConfig,
    rpc_timeout: Duration,
) -> Result<RefundFundingTopologyVerification, RouteVerifierError> {
    let mut successes = Vec::new();
    let mut last_error = None;
    for transport in transports {
        match timeout(
            rpc_timeout,
            verify_refund_funding_topology(transport.as_ref(), topology),
        )
        .await
        {
            Ok(Ok(verification)) => successes.push(verification),
            Ok(Err(error)) => last_error = Some(error.to_string()),
            Err(_) => last_error = Some("RPC verification deadline exceeded".to_string()),
        }
    }
    let Some(first) = successes.first().cloned() else {
        return Err(RouteVerifierError::ProvidersExhausted(
            last_error.unwrap_or_else(|| "no provider returned a result".to_string()),
        ));
    };
    if successes
        .iter()
        .any(|candidate| candidate.verification_digest != first.verification_digest)
    {
        return Err(RouteVerifierError::ProvidersExhausted(
            "configured providers returned conflicting finalized refund funding evidence"
                .to_string(),
        ));
    }
    Ok(first)
}

fn validate_verifier_bounds(
    transports: &[Arc<dyn JsonRpcTransport>],
    batch_size: usize,
    rpc_timeout: Duration,
) -> Result<(), RouteVerifierError> {
    if transports.is_empty() || transports.len() > MAX_PROVIDER_ATTEMPTS {
        return Err(RouteVerifierError::Configuration(
            "verifier provider pool is outside the bounded range".to_string(),
        ));
    }
    if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
        return Err(RouteVerifierError::Configuration(
            "verifier batch size is outside the bounded range".to_string(),
        ));
    }
    if !(Duration::from_millis(50)..=Duration::from_secs(10)).contains(&rpc_timeout) {
        return Err(RouteVerifierError::Configuration(
            "verifier RPC timeout is outside the bounded range".to_string(),
        ));
    }
    Ok(())
}

fn route_config(candidate: &SettlementRouteVerificationCandidate) -> SettlementRouteConfig {
    SettlementRouteConfig {
        route_id: candidate.id,
        version: candidate.version,
        merchant_account: candidate.merchant_account.clone(),
        mint: candidate.mint.clone(),
        tax_vault: candidate.tax_vault.clone(),
        net_destination: candidate.net_destination.clone(),
        token_program: candidate.token_program.clone(),
    }
}

pub async fn build_store(config: &RouteVerifierConfig) -> Result<PgStore, RouteVerifierError> {
    let store = PgStore::from_url(
        &config.database_url,
        config.database_pool_size,
        DEFAULT_POOL_WAIT_TIMEOUT,
        tax_engine::ENGINE_VERSION,
    )?;
    store.ping().await?;
    Ok(store)
}

fn required_env(name: &str) -> Result<String, RouteVerifierError> {
    let value = env::var(name)
        .map_err(|_| RouteVerifierError::Configuration(format!("{name} is required")))?;
    if value.trim().is_empty() {
        return Err(RouteVerifierError::Configuration(format!(
            "{name} must not be empty"
        )));
    }
    Ok(value)
}

fn parse_usize(name: &str, default: usize) -> Result<usize, RouteVerifierError> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            RouteVerifierError::Configuration(format!("{name} must be a positive integer"))
        }),
        Err(_) => Ok(default),
    }
}

fn parse_bool(name: &str, default: bool) -> Result<bool, RouteVerifierError> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            RouteVerifierError::Configuration(format!("{name} must be true or false"))
        }),
        Err(_) => Ok(default),
    }
}

fn parse_duration_ms(
    name: &str,
    default: Duration,
    range: std::ops::RangeInclusive<u64>,
) -> Result<Duration, RouteVerifierError> {
    let millis = match env::var(name) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            RouteVerifierError::Configuration(format!(
                "{name} must be an integer number of milliseconds"
            ))
        })?,
        Err(_) => return Ok(default),
    };
    if !range.contains(&millis) {
        return Err(RouteVerifierError::Configuration(format!(
            "{name} is outside its bounded range"
        )));
    }
    Ok(Duration::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_environment_is_explicit_and_bounded() {
        assert_eq!(
            parse_deployment_environment("local").unwrap(),
            DeploymentEnvironment::Local
        );
        assert_eq!(
            parse_deployment_environment("staging").unwrap(),
            DeploymentEnvironment::Staging
        );
        assert_eq!(
            parse_deployment_environment("production").unwrap(),
            DeploymentEnvironment::Production
        );
        for value in ["", "mainnet", "LOCAL", " production "] {
            assert!(parse_deployment_environment(value).is_err());
        }
    }

    #[test]
    fn insecure_rpc_is_only_valid_for_local_environment() {
        assert!(DeploymentEnvironment::Local.allows_insecure_rpc(true));
        assert!(DeploymentEnvironment::Local.allows_insecure_rpc(false));
        assert!(DeploymentEnvironment::Staging.allows_insecure_rpc(false));
        assert!(!DeploymentEnvironment::Staging.allows_insecure_rpc(true));
        assert!(DeploymentEnvironment::Production.allows_insecure_rpc(false));
        assert!(!DeploymentEnvironment::Production.allows_insecure_rpc(true));
    }

    #[test]
    fn route_config_preserves_the_candidate_identity() {
        let candidate = SettlementRouteVerificationCandidate {
            id: uuid::Uuid::from_u128(1),
            merchant_id: uuid::Uuid::from_u128(2),
            route_key: "route".to_string(),
            version: 4,
            merchant_account: "merchant".to_string(),
            mint: "mint".to_string(),
            tax_vault: "vault".to_string(),
            net_destination: "destination".to_string(),
            token_program: "program".to_string(),
        };
        let route = route_config(&candidate);
        assert_eq!(route.route_id, candidate.id);
        assert_eq!(route.version, candidate.version);
        assert_eq!(route.mint, candidate.mint);
    }

    fn verification(digest: [u8; 32]) -> SettlementRouteVerification {
        SettlementRouteVerification {
            route_id: uuid::Uuid::from_u128(1),
            route_version: 1,
            merchant_account: "merchant".to_string(),
            mint: "mint".to_string(),
            tax_vault: "vault".to_string(),
            net_destination: "destination".to_string(),
            token_program: "program".to_string(),
            verification_slot: 7,
            verification_commitment: "finalized",
            mint_decimals: 6,
            accounts: Vec::new(),
            verification_digest: digest,
        }
    }

    #[test]
    fn provider_agreement_rejects_conflicting_finalized_evidence() {
        assert!(select_agreed_verification(vec![verification([1; 32])], None).is_ok());
        assert!(matches!(
            select_agreed_verification(
                vec![verification([1; 32]), verification([2; 32])],
                None,
            ),
            Err(RouteVerifierError::ProvidersExhausted(message))
                if message.contains("conflicting finalized route evidence")
        ));
    }
}
