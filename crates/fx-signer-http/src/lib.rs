#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Bounded, vendor-neutral HTTP transport for a server-owned FX signer.
//!
//! This crate never holds or generates private keys. The issuer remains
//! responsible for verifying the returned signature against the configured
//! public key and the exact evidence envelope.

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use fx_provider::{AsyncFxEvidenceSigner, FxProviderError};
use reqwest::{Url, header};
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};
use thiserror::Error;

pub const MAX_ENDPOINT_BYTES: usize = 2 * 1024;
pub const MAX_BEARER_TOKEN_BYTES: usize = 4 * 1024;
pub const DEFAULT_HTTP_TIMEOUT_MILLIS: u64 = 500;
pub const MAX_HTTP_TIMEOUT_MILLIS: u64 = 5_000;
pub const MAX_REQUEST_BYTES: usize = 16 * 1024;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 4 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxSignerHttpEnvironment {
    LocalDevelopment,
    Staging,
    Production,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpFxEvidenceSignerConfig {
    pub environment: FxSignerHttpEnvironment,
    pub timeout: Duration,
    pub max_response_bytes: usize,
}

impl Default for HttpFxEvidenceSignerConfig {
    fn default() -> Self {
        Self {
            environment: FxSignerHttpEnvironment::LocalDevelopment,
            timeout: Duration::from_millis(DEFAULT_HTTP_TIMEOUT_MILLIS),
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl HttpFxEvidenceSignerConfig {
    fn validate(self) -> Result<Self, FxHttpSignerError> {
        if self.timeout.is_zero() || self.timeout > Duration::from_millis(MAX_HTTP_TIMEOUT_MILLIS) {
            return Err(FxHttpSignerError::InvalidConfiguration(
                "HTTP timeout is empty or unbounded",
            ));
        }
        if self.max_response_bytes == 0 || self.max_response_bytes > MAX_RESPONSE_BYTES {
            return Err(FxHttpSignerError::InvalidConfiguration(
                "HTTP response limit is empty or unbounded",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FxHttpSignerError {
    #[error("invalid FX signer HTTP configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("FX signer HTTP client construction failed")]
    ClientConstruction,
}

pub struct HttpFxEvidenceSigner {
    key_id: [u8; 32],
    public_key: [u8; 32],
    endpoint: Url,
    bearer_token: String,
    client: reqwest::Client,
    max_response_bytes: usize,
}

impl fmt::Debug for HttpFxEvidenceSigner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpFxEvidenceSigner")
            .field("key_id", &hex_digest(&self.key_id))
            .field("public_key", &hex_digest(&self.public_key))
            .field("endpoint", &self.endpoint)
            .field("bearer_token", &"[REDACTED]")
            .field("max_response_bytes", &self.max_response_bytes)
            .finish_non_exhaustive()
    }
}

impl HttpFxEvidenceSigner {
    pub fn new(
        key_id: [u8; 32],
        public_key: [u8; 32],
        endpoint: &str,
        bearer_token: &str,
        config: HttpFxEvidenceSignerConfig,
    ) -> Result<Self, FxHttpSignerError> {
        if key_id == [0; 32] || public_key == [0; 32] {
            return Err(FxHttpSignerError::InvalidConfiguration(
                "signer identity must be non-zero",
            ));
        }
        if endpoint.is_empty() || endpoint.len() > MAX_ENDPOINT_BYTES {
            return Err(FxHttpSignerError::InvalidConfiguration(
                "HTTP endpoint is empty or unbounded",
            ));
        }
        if bearer_token.is_empty() || bearer_token.len() > MAX_BEARER_TOKEN_BYTES {
            return Err(FxHttpSignerError::InvalidConfiguration(
                "bearer token is empty or unbounded",
            ));
        }
        let config = config.validate()?;
        let endpoint_url = Url::parse(endpoint).map_err(|_| {
            FxHttpSignerError::InvalidConfiguration("HTTP endpoint is not a valid URL")
        })?;
        validate_endpoint(&endpoint_url, config.environment)?;

        let client = reqwest::Client::builder()
            .connect_timeout(config.timeout)
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .https_only(!matches!(
                config.environment,
                FxSignerHttpEnvironment::LocalDevelopment
            ))
            .build()
            .map_err(|_| FxHttpSignerError::ClientConstruction)?;

        Ok(Self {
            key_id,
            public_key,
            endpoint: endpoint_url,
            bearer_token: bearer_token.to_string(),
            client,
            max_response_bytes: config.max_response_bytes,
        })
    }
}

#[async_trait]
impl AsyncFxEvidenceSigner for HttpFxEvidenceSigner {
    fn key_id(&self) -> [u8; 32] {
        self.key_id
    }

    fn public_key(&self) -> [u8; 32] {
        self.public_key
    }

    async fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, FxProviderError> {
        let request = SignRequest {
            key_id: hex_digest(&self.key_id),
            payload: STANDARD.encode(payload),
        };
        let request_bytes =
            serde_json::to_vec(&request).map_err(|_| FxProviderError::SignerFailed)?;
        if request_bytes.len() > MAX_REQUEST_BYTES {
            return Err(FxProviderError::SignerFailed);
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
            .map_err(|_| FxProviderError::SignerFailed)?;
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
            return Err(FxProviderError::SignerFailed);
        }

        let body = read_bounded_body(response, self.max_response_bytes).await?;
        let response: SignResponse =
            serde_json::from_slice(&body).map_err(|_| FxProviderError::SignerFailed)?;
        let signature = STANDARD
            .decode(&response.signature)
            .map_err(|_| FxProviderError::SignerFailed)?;
        if signature.len() != 64 || STANDARD.encode(&signature) != response.signature {
            return Err(FxProviderError::SignerFailed);
        }
        Ok(signature)
    }
}

#[derive(Serialize)]
struct SignRequest {
    key_id: String,
    payload: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignResponse {
    signature: String,
}

async fn read_bounded_body(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, FxProviderError> {
    let mut body = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| FxProviderError::SignerFailed)?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(FxProviderError::SignerFailed);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn validate_endpoint(
    endpoint: &Url,
    environment: FxSignerHttpEnvironment,
) -> Result<(), FxHttpSignerError> {
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(FxHttpSignerError::InvalidConfiguration(
            "HTTP endpoint contains credentials or mutable URL components",
        ));
    }
    let host = endpoint
        .host_str()
        .ok_or(FxHttpSignerError::InvalidConfiguration(
            "HTTP endpoint has no host",
        ))?;
    match environment {
        FxSignerHttpEnvironment::LocalDevelopment => {
            if !is_loopback_host(host) || endpoint.port().is_none() {
                return Err(FxHttpSignerError::InvalidConfiguration(
                    "local HTTP endpoint must be an explicit loopback address",
                ));
            }
            if endpoint.scheme() != "http" && endpoint.scheme() != "https" {
                return Err(FxHttpSignerError::InvalidConfiguration(
                    "local HTTP endpoint must use HTTP or HTTPS",
                ));
            }
        }
        FxSignerHttpEnvironment::Staging | FxSignerHttpEnvironment::Production => {
            if endpoint.scheme() != "https" {
                return Err(FxHttpSignerError::InvalidConfiguration(
                    "staging and production signer endpoints require HTTPS",
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
        let high = byte >> 4;
        let low = byte & 0x0f;
        output.push(b"0123456789abcdef"[usize::from(high)] as char);
        output.push(b"0123456789abcdef"[usize::from(low)] as char);
    }
    output
}
