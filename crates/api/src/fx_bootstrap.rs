#![deny(unsafe_code)]

use crate::config::DeploymentEnvironment;
use fx_provider::{
    AsyncFxEvidenceIssuance, ConfiguredFxEvidenceIssuance, ExactRate, FxEvidenceCoordinator,
    FxProviderAcquisitionConfig, FxProviderError, FxProviderPolicyRegistry, FxSigningConfig,
    MAX_ALLOWED_PROVIDERS,
};
use fx_provider_http::{FxHttpEnvironment, HttpFxQuoteSource, HttpFxQuoteSourceConfig};
use fx_signer_http::{FxSignerHttpEnvironment, HttpFxEvidenceSigner, HttpFxEvidenceSignerConfig};
use serde::Deserialize;
use std::{collections::HashMap, env, fs::File, io::Read, sync::Arc};
use thiserror::Error;

const MAX_SOURCE_DOCUMENT_BYTES: usize = 16 * 1024;
const MAX_SECRET_ENV_NAME_BYTES: usize = 256;

#[derive(Debug, Error)]
pub enum FxBootstrapError {
    #[error("invalid FX bootstrap configuration: {0}")]
    Invalid(String),
    #[error("non-local FX activation remains gated by provider, legal, custody, and SLO decisions")]
    NonLocalActivation,
    #[error("FX policy registry configuration failed: {0}")]
    Policy(#[from] FxProviderError),
    #[error("FX provider HTTP configuration failed: {0}")]
    ProviderHttp(#[from] fx_provider_http::FxHttpProviderError),
    #[error("FX signer HTTP configuration failed: {0}")]
    SignerHttp(#[from] fx_signer_http::FxHttpSignerError),
}

#[derive(Clone)]
pub(crate) struct FxBootstrapInput {
    pub(crate) environment: FxHttpEnvironment,
    pub(crate) policy_registry_document: Option<String>,
    pub(crate) policy_digest_hex: Option<String>,
    pub(crate) provider_sources_json: Option<String>,
    pub(crate) signer_endpoint: Option<String>,
    pub(crate) signer_bearer_token: Option<String>,
    pub(crate) signer_key_id_hex: Option<String>,
    pub(crate) signer_public_key_hex: Option<String>,
    pub(crate) reference_rate_numerator: Option<String>,
    pub(crate) reference_rate_denominator: Option<String>,
    pub(crate) provider_secrets: HashMap<String, String>,
}

impl FxBootstrapInput {
    #[cfg(test)]
    pub(crate) fn absent(environment: FxHttpEnvironment) -> Self {
        Self {
            environment,
            policy_registry_document: None,
            policy_digest_hex: None,
            provider_sources_json: None,
            signer_endpoint: None,
            signer_bearer_token: None,
            signer_key_id_hex: None,
            signer_public_key_hex: None,
            reference_rate_numerator: None,
            reference_rate_denominator: None,
            provider_secrets: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderSourceDocument {
    sources: Vec<ProviderSourceConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderSourceConfig {
    provider_id: String,
    endpoint: String,
    bearer_token_env: String,
}

pub fn build_from_environment(
    environment: DeploymentEnvironment,
) -> Result<Option<Arc<dyn AsyncFxEvidenceIssuance>>, FxBootstrapError> {
    let policy_json = env_value("API_FX_POLICY_REGISTRY_JSON")?;
    let policy_file = env_value("API_FX_POLICY_REGISTRY_FILE")?;
    if policy_json.is_some() && policy_file.is_some() {
        return Err(FxBootstrapError::Invalid(
            "API_FX_POLICY_REGISTRY_JSON and API_FX_POLICY_REGISTRY_FILE are mutually exclusive"
                .to_string(),
        ));
    }
    let provider_sources_json = env_value("API_FX_PROVIDER_SOURCES_JSON")?;
    let policy_digest_hex = env_value("API_FX_POLICY_DIGEST_HEX")?;
    let signer_endpoint = env_value("API_FX_SIGNER_ENDPOINT")?;
    let signer_bearer_token = env_value("API_FX_SIGNER_BEARER_TOKEN")?;
    let signer_key_id_hex = env_value("API_FX_SIGNER_KEY_ID_HEX")?;
    let signer_public_key_hex = env_value("API_FX_SIGNER_PUBLIC_KEY_HEX")?;
    let reference_rate_numerator = env_value("API_FX_REFERENCE_RATE_NUMERATOR")?;
    let reference_rate_denominator = env_value("API_FX_REFERENCE_RATE_DENOMINATOR")?;

    let any_configuration = [
        policy_json.as_ref(),
        policy_file.as_ref(),
        policy_digest_hex.as_ref(),
        provider_sources_json.as_ref(),
        signer_endpoint.as_ref(),
        signer_bearer_token.as_ref(),
        signer_key_id_hex.as_ref(),
        signer_public_key_hex.as_ref(),
        reference_rate_numerator.as_ref(),
        reference_rate_denominator.as_ref(),
    ]
    .into_iter()
    .flatten()
    .next()
    .is_some();
    if !any_configuration {
        return Ok(None);
    }

    let policy_registry_document = match (policy_json, policy_file) {
        (Some(document), None) => Some(document),
        (None, Some(path)) => Some(read_bounded_file(&path)?),
        (None, None) => None,
        (Some(_), Some(_)) => {
            return Err(FxBootstrapError::Invalid(
                "FX policy registry sources are mutually exclusive".to_string(),
            ));
        }
    };

    let source_secrets = match provider_sources_json.as_deref() {
        Some(document) => resolve_provider_secrets(document)?,
        None => HashMap::new(),
    };
    build_from_input(FxBootstrapInput {
        environment: http_environment(environment),
        policy_registry_document,
        policy_digest_hex,
        provider_sources_json,
        signer_endpoint,
        signer_bearer_token,
        signer_key_id_hex,
        signer_public_key_hex,
        reference_rate_numerator,
        reference_rate_denominator,
        provider_secrets: source_secrets,
    })
}

pub(crate) fn build_from_input(
    input: FxBootstrapInput,
) -> Result<Option<Arc<dyn AsyncFxEvidenceIssuance>>, FxBootstrapError> {
    let any_configuration = [
        input.policy_registry_document.as_ref(),
        input.policy_digest_hex.as_ref(),
        input.provider_sources_json.as_ref(),
        input.signer_endpoint.as_ref(),
        input.signer_bearer_token.as_ref(),
        input.signer_key_id_hex.as_ref(),
        input.signer_public_key_hex.as_ref(),
        input.reference_rate_numerator.as_ref(),
        input.reference_rate_denominator.as_ref(),
    ]
    .into_iter()
    .flatten()
    .next()
    .is_some();
    if !any_configuration {
        return Ok(None);
    }

    let policy_document = required(input.policy_registry_document, "policy registry")?;
    let policy_digest_hex = required(input.policy_digest_hex, "policy digest")?;
    let provider_sources_json = required(input.provider_sources_json, "provider sources")?;
    let signer_endpoint = required(input.signer_endpoint, "signer endpoint")?;
    let signer_bearer_token = required(input.signer_bearer_token, "signer bearer token")?;
    let signer_key_id_hex = required(input.signer_key_id_hex, "signer key ID")?;
    let signer_public_key_hex = required(input.signer_public_key_hex, "signer public key")?;
    if input.reference_rate_numerator.is_some() != input.reference_rate_denominator.is_some() {
        return Err(FxBootstrapError::Invalid(
            "reference rate numerator and denominator must be configured together".to_string(),
        ));
    }
    if !matches!(input.environment, FxHttpEnvironment::LocalDevelopment) {
        return Err(FxBootstrapError::NonLocalActivation);
    }

    let policy_digest = parse_digest(&policy_digest_hex, "policy digest")?;
    let registry = FxProviderPolicyRegistry::from_json(&policy_document)?;
    let policy = registry
        .policy(policy_digest)
        .ok_or_else(|| FxBootstrapError::Invalid("policy digest is not published".to_string()))?;
    let source_document = parse_source_document(&provider_sources_json)?;
    if source_document.sources.is_empty() || source_document.sources.len() > MAX_ALLOWED_PROVIDERS {
        return Err(FxBootstrapError::Invalid(
            "provider source topology is empty or unbounded".to_string(),
        ));
    }
    if source_document.sources.len() < usize::from(policy.minimum_provider_agreement) {
        return Err(FxBootstrapError::Invalid(
            "provider source topology is smaller than policy agreement".to_string(),
        ));
    }

    let http_provider_config = HttpFxQuoteSourceConfig {
        environment: FxHttpEnvironment::LocalDevelopment,
        ..HttpFxQuoteSourceConfig::default()
    };
    let mut sources = Vec::with_capacity(source_document.sources.len());
    for (index, source) in source_document.sources.into_iter().enumerate() {
        if !policy
            .allowed_providers
            .iter()
            .any(|allowed| allowed == &source.provider_id)
        {
            return Err(FxBootstrapError::Invalid(format!(
                "provider source {index} is not in the selected policy allowlist"
            )));
        }
        let bearer_token = input
            .provider_secrets
            .get(&source.bearer_token_env)
            .ok_or_else(|| {
                FxBootstrapError::Invalid(format!(
                    "provider source {index} bearer token environment value is missing"
                ))
            })?;
        if bearer_token.trim().is_empty() {
            return Err(FxBootstrapError::Invalid(format!(
                "provider source {index} bearer token is empty"
            )));
        }
        let source = HttpFxQuoteSource::new(
            &source.provider_id,
            &source.endpoint,
            bearer_token,
            http_provider_config,
        )?;
        sources.push(Arc::new(source) as Arc<dyn fx_provider::AsyncFxQuoteSource>);
    }

    let signer = HttpFxEvidenceSigner::new(
        parse_digest(&signer_key_id_hex, "signer key ID")?,
        parse_digest(&signer_public_key_hex, "signer public key")?,
        &signer_endpoint,
        &signer_bearer_token,
        HttpFxEvidenceSignerConfig {
            environment: FxSignerHttpEnvironment::LocalDevelopment,
            ..HttpFxEvidenceSignerConfig::default()
        },
    )?;
    let reference_rate = match (
        input.reference_rate_numerator,
        input.reference_rate_denominator,
    ) {
        (Some(numerator), Some(denominator)) => Some(ExactRate {
            numerator: parse_positive_i128(&numerator, "reference rate numerator")?,
            denominator: parse_positive_i128(&denominator, "reference rate denominator")?,
        }),
        (None, None) => None,
        _ => {
            return Err(FxBootstrapError::Invalid(
                "reference rate numerator and denominator must be configured together".to_string(),
            ));
        }
    };
    let coordinator = Arc::new(FxEvidenceCoordinator::new(
        registry,
        sources,
        Arc::new(signer),
        FxProviderAcquisitionConfig::default(),
        FxSigningConfig::default(),
    ));
    let issuance = ConfiguredFxEvidenceIssuance::new(coordinator, policy_digest, reference_rate)?;
    Ok(Some(Arc::new(issuance)))
}

fn parse_source_document(document: &str) -> Result<ProviderSourceDocument, FxBootstrapError> {
    if document.len() > MAX_SOURCE_DOCUMENT_BYTES {
        return Err(FxBootstrapError::Invalid(
            "provider source document exceeds its bounded size".to_string(),
        ));
    }
    let document: ProviderSourceDocument = serde_json::from_str(document).map_err(|_| {
        FxBootstrapError::Invalid("provider source document is invalid".to_string())
    })?;
    if document.sources.len() > MAX_ALLOWED_PROVIDERS {
        return Err(FxBootstrapError::Invalid(
            "provider source document contains too many sources".to_string(),
        ));
    }
    for (index, source) in document.sources.iter().enumerate() {
        if source.provider_id.trim().is_empty()
            || source.endpoint.trim().is_empty()
            || source.bearer_token_env.trim().is_empty()
        {
            return Err(FxBootstrapError::Invalid(format!(
                "provider source {index} contains an empty field"
            )));
        }
        if source.bearer_token_env.len() > MAX_SECRET_ENV_NAME_BYTES
            || source
                .bearer_token_env
                .bytes()
                .any(|byte| byte == b'=' || byte == 0)
        {
            return Err(FxBootstrapError::Invalid(format!(
                "provider source {index} bearer token environment name is invalid"
            )));
        }
        if document.sources[..index]
            .iter()
            .any(|previous| previous.provider_id == source.provider_id)
        {
            return Err(FxBootstrapError::Invalid(
                "provider source topology contains duplicate providers".to_string(),
            ));
        }
    }
    Ok(document)
}

fn resolve_provider_secrets(document: &str) -> Result<HashMap<String, String>, FxBootstrapError> {
    let source_document = parse_source_document(document)?;
    let mut secrets = HashMap::with_capacity(source_document.sources.len());
    for source in source_document.sources {
        let value = env::var(&source.bearer_token_env).map_err(|_| {
            FxBootstrapError::Invalid(format!(
                "provider bearer token environment value for source {} is missing",
                source.provider_id
            ))
        })?;
        secrets.insert(source.bearer_token_env, value);
    }
    Ok(secrets)
}

fn read_bounded_file(path: &str) -> Result<String, FxBootstrapError> {
    if path.trim().is_empty() || path.len() > 4096 {
        return Err(FxBootstrapError::Invalid(
            "FX policy registry file path is empty or unbounded".to_string(),
        ));
    }
    let mut file = File::open(path).map_err(|_| {
        FxBootstrapError::Invalid("FX policy registry file cannot be opened".to_string())
    })?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(fx_provider::MAX_POLICY_REGISTRY_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            FxBootstrapError::Invalid("FX policy registry file cannot be read".to_string())
        })?;
    if bytes.len() > fx_provider::MAX_POLICY_REGISTRY_DOCUMENT_BYTES {
        return Err(FxBootstrapError::Invalid(
            "FX policy registry file exceeds its bounded size".to_string(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| FxBootstrapError::Invalid("FX policy registry file is not UTF-8".to_string()))
}

fn env_value(name: &str) -> Result<Option<String>, FxBootstrapError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(FxBootstrapError::Invalid(format!(
            "{name} must be valid UTF-8"
        ))),
    }
}

fn required(value: Option<String>, label: &str) -> Result<String, FxBootstrapError> {
    value.ok_or_else(|| {
        FxBootstrapError::Invalid(format!("FX configuration is incomplete: {label}"))
    })
}

fn parse_digest(value: &str, label: &str) -> Result<[u8; 32], FxBootstrapError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(FxBootstrapError::Invalid(format!(
            "{label} must be exactly 32 lowercase hexadecimal bytes"
        )));
    }
    let mut digest = [0_u8; 32];
    let bytes = value.as_bytes();
    for (index, byte) in digest.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = (hex_nibble(bytes[offset])? << 4) | hex_nibble(bytes[offset + 1])?;
    }
    if digest == [0; 32] {
        return Err(FxBootstrapError::Invalid(format!(
            "{label} must be non-zero"
        )));
    }
    Ok(digest)
}

fn hex_nibble(value: u8) -> Result<u8, FxBootstrapError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(FxBootstrapError::Invalid(
            "hexadecimal value is invalid".to_string(),
        )),
    }
}

fn parse_positive_i128(value: &str, label: &str) -> Result<i128, FxBootstrapError> {
    let parsed = value.parse::<i128>().map_err(|_| {
        FxBootstrapError::Invalid(format!("{label} must be a positive decimal integer"))
    })?;
    if parsed <= 0 {
        return Err(FxBootstrapError::Invalid(format!(
            "{label} must be a positive decimal integer"
        )));
    }
    Ok(parsed)
}

fn http_environment(environment: DeploymentEnvironment) -> FxHttpEnvironment {
    match environment {
        DeploymentEnvironment::Local => FxHttpEnvironment::LocalDevelopment,
        DeploymentEnvironment::Staging => FxHttpEnvironment::Staging,
        DeploymentEnvironment::Production => FxHttpEnvironment::Production,
    }
}

#[cfg(test)]
mod tests {
    use super::{FxBootstrapInput, build_from_input};
    use fx_provider_http::FxHttpEnvironment;
    use std::collections::HashMap;

    fn policy_document() -> String {
        serde_json::json!({
            "policies": [{
                "policy_digest": vec![6; 32],
                "allowed_pairs": [{"source_currency": "USD", "target_currency": "EUR"}],
                "allowed_providers": ["provider-a"],
                "minimum_provider_agreement": 1,
                "max_observation_age_seconds": 600,
                "max_future_skew_seconds": 60,
                "max_quote_lifetime_seconds": 900,
                "require_legal_acceptance": true,
                "max_deviation_basis_points": 1000
            }]
        })
        .to_string()
    }

    fn complete_input(environment: FxHttpEnvironment) -> FxBootstrapInput {
        let mut provider_secrets = HashMap::new();
        provider_secrets.insert(
            "FX_PROVIDER_A_TOKEN".to_string(),
            "provider-token".to_string(),
        );
        FxBootstrapInput {
            environment,
            policy_registry_document: Some(policy_document()),
            policy_digest_hex: Some("06".repeat(32)),
            provider_sources_json: Some(
                r#"{"sources":[{"provider_id":"provider-a","endpoint":"http://127.0.0.1:18080/quote","bearer_token_env":"FX_PROVIDER_A_TOKEN"}]}"#.to_string(),
            ),
            signer_endpoint: Some("http://127.0.0.1:18081/sign".to_string()),
            signer_bearer_token: Some("signer-token".to_string()),
            signer_key_id_hex: Some("07".repeat(32)),
            signer_public_key_hex: Some("08".repeat(32)),
            reference_rate_numerator: None,
            reference_rate_denominator: None,
            provider_secrets,
        }
    }

    #[test]
    fn absent_configuration_keeps_fx_issuance_disabled() {
        let input = FxBootstrapInput::absent(FxHttpEnvironment::LocalDevelopment);
        assert!(
            build_from_input(input)
                .expect("absent FX configuration is valid")
                .is_none()
        );
    }

    #[test]
    fn partial_configuration_fails_closed() {
        let mut input = FxBootstrapInput::absent(FxHttpEnvironment::LocalDevelopment);
        input.policy_digest_hex = Some("06".repeat(32));
        let error = match build_from_input(input) {
            Ok(_) => panic!("partial configuration must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("incomplete"));
    }

    #[test]
    fn complete_local_configuration_constructs_the_server_owned_port() {
        let issuance = build_from_input(complete_input(FxHttpEnvironment::LocalDevelopment))
            .expect("local FX bootstrap")
            .expect("complete configuration enables local FX");
        assert!(std::sync::Arc::strong_count(&issuance) >= 1);
    }

    #[test]
    fn non_local_configuration_is_rejected_before_activation() {
        let error = match build_from_input(complete_input(FxHttpEnvironment::Staging)) {
            Ok(_) => panic!("non-local activation must remain gated"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("non-local"));
    }

    #[test]
    fn source_secret_indirection_and_strict_topology_are_required() {
        let mut missing_secret = complete_input(FxHttpEnvironment::LocalDevelopment);
        missing_secret.provider_secrets.clear();
        let error = match build_from_input(missing_secret) {
            Ok(_) => panic!("missing secret must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("bearer token"));

        let mut duplicate = complete_input(FxHttpEnvironment::LocalDevelopment);
        duplicate.provider_sources_json = Some(
            r#"{"sources":[{"provider_id":"provider-a","endpoint":"http://127.0.0.1:18080/quote","bearer_token_env":"FX_PROVIDER_A_TOKEN"},{"provider_id":"provider-a","endpoint":"http://127.0.0.1:18082/quote","bearer_token_env":"FX_PROVIDER_A_TOKEN"}]}"#.to_string(),
        );
        let error = match build_from_input(duplicate) {
            Ok(_) => panic!("duplicate providers must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("duplicate"));
    }

    #[test]
    fn exact_digest_and_reference_rate_parsing_is_bounded() {
        let mut input = complete_input(FxHttpEnvironment::LocalDevelopment);
        input.policy_digest_hex = Some("06".repeat(31));
        assert!(build_from_input(input).is_err());

        let mut input = complete_input(FxHttpEnvironment::LocalDevelopment);
        input.reference_rate_numerator = Some("9".to_string());
        assert!(build_from_input(input).is_err());

        let mut input = complete_input(FxHttpEnvironment::LocalDevelopment);
        input.reference_rate_numerator = Some("9".to_string());
        input.reference_rate_denominator = Some("8".to_string());
        assert!(build_from_input(input).is_ok());
    }
}
