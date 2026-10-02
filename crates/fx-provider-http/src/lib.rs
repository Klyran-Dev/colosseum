#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Bounded, vendor-neutral HTTP transport for server-owned FX quote sources.
//!
//! This crate deliberately does not choose a provider, legal owner, custody
//! model, or production route. It turns one configured HTTP response into the
//! existing `fx_provider::AsyncFxQuoteSource` port with strict limits.

use async_trait::async_trait;
use fx_provider::{
    AsyncFxQuoteSource, FxProviderError, FxQuoteCandidate, FxQuoteRequest, LegalAcceptance,
};
use reqwest::{Url, header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, time::Duration};
use tax_engine::RoundingMode;
use thiserror::Error;

pub const MAX_ENDPOINT_BYTES: usize = 2 * 1024;
pub const MAX_BEARER_TOKEN_BYTES: usize = 4 * 1024;
pub const DEFAULT_HTTP_TIMEOUT_MILLIS: u64 = 500;
pub const MAX_HTTP_TIMEOUT_MILLIS: u64 = 5_000;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 32 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 32 * 1024;
pub const MAX_REQUEST_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxHttpEnvironment {
    LocalDevelopment,
    Staging,
    Production,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpFxQuoteSourceConfig {
    pub environment: FxHttpEnvironment,
    pub timeout: Duration,
    pub max_response_bytes: usize,
}

impl Default for HttpFxQuoteSourceConfig {
    fn default() -> Self {
        Self {
            environment: FxHttpEnvironment::LocalDevelopment,
            timeout: Duration::from_millis(DEFAULT_HTTP_TIMEOUT_MILLIS),
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl HttpFxQuoteSourceConfig {
    fn validate(self) -> Result<Self, FxHttpProviderError> {
        if self.timeout.is_zero() || self.timeout > Duration::from_millis(MAX_HTTP_TIMEOUT_MILLIS) {
            return Err(FxHttpProviderError::InvalidConfiguration(
                "HTTP timeout is empty or unbounded",
            ));
        }
        if self.max_response_bytes == 0 || self.max_response_bytes > MAX_RESPONSE_BYTES {
            return Err(FxHttpProviderError::InvalidConfiguration(
                "HTTP response limit is empty or unbounded",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FxHttpProviderError {
    #[error("invalid FX HTTP source configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("FX HTTP client construction failed")]
    ClientConstruction,
}

pub struct HttpFxQuoteSource {
    provider_id: String,
    endpoint: Url,
    bearer_token: String,
    client: reqwest::Client,
    max_response_bytes: usize,
}

impl fmt::Debug for HttpFxQuoteSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpFxQuoteSource")
            .field("provider_id", &self.provider_id)
            .field("endpoint", &self.endpoint)
            .field("bearer_token", &"[REDACTED]")
            .field("max_response_bytes", &self.max_response_bytes)
            .finish_non_exhaustive()
    }
}

impl HttpFxQuoteSource {
    pub fn new(
        provider_id: &str,
        endpoint: &str,
        bearer_token: &str,
        config: HttpFxQuoteSourceConfig,
    ) -> Result<Self, FxHttpProviderError> {
        if provider_id.trim().is_empty() || provider_id.len() > 64 {
            return Err(FxHttpProviderError::InvalidConfiguration(
                "provider identity is empty or unbounded",
            ));
        }
        if endpoint.is_empty() || endpoint.len() > MAX_ENDPOINT_BYTES {
            return Err(FxHttpProviderError::InvalidConfiguration(
                "HTTP endpoint is empty or unbounded",
            ));
        }
        if bearer_token.is_empty() || bearer_token.len() > MAX_BEARER_TOKEN_BYTES {
            return Err(FxHttpProviderError::InvalidConfiguration(
                "bearer token is empty or unbounded",
            ));
        }
        let config = config.validate()?;
        let endpoint_url = Url::parse(endpoint).map_err(|_| {
            FxHttpProviderError::InvalidConfiguration("HTTP endpoint is not a valid URL")
        })?;
        validate_endpoint(&endpoint_url, config.environment)?;

        let client = reqwest::Client::builder()
            .connect_timeout(config.timeout)
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .https_only(!matches!(
                config.environment,
                FxHttpEnvironment::LocalDevelopment
            ))
            .build()
            .map_err(|_| FxHttpProviderError::ClientConstruction)?;

        Ok(Self {
            provider_id: provider_id.to_string(),
            endpoint: endpoint_url,
            bearer_token: bearer_token.to_string(),
            client,
            max_response_bytes: config.max_response_bytes,
        })
    }
}

#[async_trait]
impl AsyncFxQuoteSource for HttpFxQuoteSource {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn quote(&self, request: &FxQuoteRequest) -> Result<FxQuoteCandidate, FxProviderError> {
        let request_wire = QuoteRequestWire::from_request(request);
        let request_bytes =
            serde_json::to_vec(&request_wire).map_err(|_| FxProviderError::ProviderSourceFailed)?;
        if request_bytes.len() > MAX_REQUEST_BYTES {
            return Err(FxProviderError::ProviderSourceFailed);
        }

        let response = self
            .client
            .post(self.endpoint.clone())
            .bearer_auth(&self.bearer_token)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .body(request_bytes)
            .send()
            .await
            .map_err(|_| FxProviderError::ProviderSourceFailed)?;

        if !response.status().is_success()
            || !response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| {
                    value
                        .split(';')
                        .next()
                        .is_some_and(|media_type| media_type.trim() == "application/json")
                })
            || response
                .content_length()
                .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(FxProviderError::ProviderSourceFailed);
        }

        let body = read_bounded_body(response, self.max_response_bytes).await?;
        let response_wire: QuoteResponseWire =
            serde_json::from_slice(&body).map_err(|_| FxProviderError::ProviderSourceFailed)?;
        response_wire.into_candidate(request, &self.provider_id, Sha256::digest(&body).into())
    }
}

#[derive(Serialize)]
struct QuoteRequestWire {
    source_currency: String,
    target_currency: String,
    merchant_digest: String,
    facts_digest: String,
}

impl QuoteRequestWire {
    fn from_request(request: &FxQuoteRequest) -> Self {
        Self {
            source_currency: request.source_currency.clone(),
            target_currency: request.target_currency.clone(),
            merchant_digest: hex_digest(&request.merchant_id),
            facts_digest: hex_digest(&request.facts_digest),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuoteResponseWire {
    source_currency: String,
    target_currency: String,
    numerator: String,
    denominator: String,
    rounding_mode: RoundingMode,
    quantum_atomic: String,
    quote_id: String,
    observed_at_unix: i64,
    expires_at_unix: i64,
    legal_acceptance: LegalAcceptanceWire,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum LegalAcceptanceWire {
    Approved { authority_digest: String },
    NotApproved,
    Rejected,
}

impl QuoteResponseWire {
    fn into_candidate(
        self,
        request: &FxQuoteRequest,
        provider_id: &str,
        evidence_digest: [u8; 32],
    ) -> Result<FxQuoteCandidate, FxProviderError> {
        if self.source_currency != request.source_currency
            || self.target_currency != request.target_currency
        {
            return Err(FxProviderError::ProviderSourceFailed);
        }
        let legal_acceptance = match self.legal_acceptance {
            LegalAcceptanceWire::Approved { authority_digest } => LegalAcceptance::Approved {
                authority_digest: parse_digest(&authority_digest)?,
            },
            LegalAcceptanceWire::NotApproved => LegalAcceptance::NotApproved,
            LegalAcceptanceWire::Rejected => LegalAcceptance::Rejected,
        };
        Ok(FxQuoteCandidate {
            source_currency: self.source_currency,
            target_currency: self.target_currency,
            numerator: parse_i128(&self.numerator)?,
            denominator: parse_i128(&self.denominator)?,
            rounding_mode: self.rounding_mode,
            quantum_atomic: parse_i128(&self.quantum_atomic)?,
            provider: provider_id.to_string(),
            quote_id: self.quote_id,
            observed_at_unix: self.observed_at_unix,
            expires_at_unix: self.expires_at_unix,
            evidence_digest,
            legal_acceptance,
        })
    }
}

async fn read_bounded_body(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, FxProviderError> {
    let mut body = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| FxProviderError::ProviderSourceFailed)?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(FxProviderError::ProviderSourceFailed);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn validate_endpoint(
    endpoint: &Url,
    environment: FxHttpEnvironment,
) -> Result<(), FxHttpProviderError> {
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(FxHttpProviderError::InvalidConfiguration(
            "HTTP endpoint contains credentials or mutable URL components",
        ));
    }
    let host = endpoint
        .host_str()
        .ok_or(FxHttpProviderError::InvalidConfiguration(
            "HTTP endpoint has no host",
        ))?;
    match environment {
        FxHttpEnvironment::LocalDevelopment => {
            if !is_loopback_host(host) || endpoint.port().is_none() {
                return Err(FxHttpProviderError::InvalidConfiguration(
                    "local HTTP endpoint must be an explicit loopback address",
                ));
            }
            if endpoint.scheme() != "http" && endpoint.scheme() != "https" {
                return Err(FxHttpProviderError::InvalidConfiguration(
                    "local HTTP endpoint must use HTTP or HTTPS",
                ));
            }
        }
        FxHttpEnvironment::Staging | FxHttpEnvironment::Production => {
            if endpoint.scheme() != "https" {
                return Err(FxHttpProviderError::InvalidConfiguration(
                    "staging and production FX endpoints require HTTPS",
                ));
            }
        }
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

fn hex_digest(digest: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        output.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    output
}

fn parse_digest(value: &str) -> Result<[u8; 32], FxProviderError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(FxProviderError::ProviderSourceFailed);
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks(2).enumerate() {
        digest[index] = (hex_nibble(chunk[0])? << 4) | hex_nibble(chunk[1])?;
    }
    if value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(FxProviderError::ProviderSourceFailed);
    }
    Ok(digest)
}

fn hex_nibble(byte: u8) -> Result<u8, FxProviderError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(FxProviderError::ProviderSourceFailed),
    }
}

fn parse_i128(value: &str) -> Result<i128, FxProviderError> {
    let (sign, digits) = value
        .strip_prefix('-')
        .map_or((None, value), |digits| (Some('-'), digits));
    if digits.is_empty()
        || value.trim() != value
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || sign.is_none() && value.starts_with('+')
    {
        return Err(FxProviderError::ProviderSourceFailed);
    }
    value
        .parse::<i128>()
        .map_err(|_| FxProviderError::ProviderSourceFailed)
}
