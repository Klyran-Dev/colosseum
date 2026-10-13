#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use thiserror::Error;
use uuid::Uuid;

pub const OTP_LENGTH: usize = 6;
pub const TOKEN_TTL_SECONDS: i64 = 900;
pub const MAX_EMAIL_BYTES: usize = 320;
pub const MAX_TOKEN_CLAIM_BYTES: usize = 128;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AuthDomainError {
    #[error("email is empty")]
    EmptyEmail,
    #[error("email is too long")]
    EmailTooLong,
    #[error("email contains invalid characters")]
    InvalidEmail,
    #[error("OTP must contain exactly six decimal digits")]
    InvalidOtp,
    #[error("authentication intent must be signin or signup")]
    InvalidIntent,
    #[error("secure random number generation failed")]
    Randomness,
    #[error("authentication configuration is invalid: {0}")]
    InvalidConfiguration(String),
    #[error("token claim serialization failed")]
    Serialization,
    #[error("access token is invalid")]
    InvalidToken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthIntent {
    SignIn,
    SignUp,
}

pub fn parse_auth_intent(value: Option<&str>) -> Result<AuthIntent, AuthDomainError> {
    match value.unwrap_or("signin") {
        "signin" => Ok(AuthIntent::SignIn),
        "signup" => Ok(AuthIntent::SignUp),
        _ => Err(AuthDomainError::InvalidIntent),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OtpPolicy {
    pub ttl_seconds: i64,
    pub resend_cooldown_seconds: i64,
    pub rate_limit_window_seconds: i64,
    pub max_attempts: i32,
    pub max_email_requests: i32,
    pub max_source_requests: i32,
}

impl Default for OtpPolicy {
    fn default() -> Self {
        Self {
            ttl_seconds: 300,
            resend_cooldown_seconds: 60,
            rate_limit_window_seconds: 900,
            max_attempts: 5,
            max_email_requests: 3,
            max_source_requests: 20,
        }
    }
}

pub fn normalize_email(input: &str) -> Result<String, AuthDomainError> {
    let value = input.trim();
    if value.is_empty() {
        return Err(AuthDomainError::EmptyEmail);
    }
    if value.len() > MAX_EMAIL_BYTES {
        return Err(AuthDomainError::EmailTooLong);
    }
    if !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(AuthDomainError::InvalidEmail);
    }
    let Some((local, domain)) = value.split_once('@') else {
        return Err(AuthDomainError::InvalidEmail);
    };
    if local.is_empty()
        || local.len() > 64
        || domain.is_empty()
        || domain.len() > 255
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
        || domain.contains("..")
    {
        return Err(AuthDomainError::InvalidEmail);
    }
    Ok(value.to_ascii_lowercase())
}

pub fn validate_otp_code(code: &str) -> Result<(), AuthDomainError> {
    if code.len() != OTP_LENGTH || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AuthDomainError::InvalidOtp);
    }
    Ok(())
}

pub fn generate_otp() -> Result<String, AuthDomainError> {
    let bucket = 1_000_000_u64;
    let limit = (u64::from(u32::MAX) + 1) / bucket * bucket;
    for _ in 0..16 {
        let mut bytes = [0_u8; 4];
        getrandom::fill(&mut bytes).map_err(|_| AuthDomainError::Randomness)?;
        let value = u64::from(u32::from_le_bytes(bytes));
        if value < limit {
            return Ok(format!("{:06}", value % bucket));
        }
    }
    Err(AuthDomainError::Randomness)
}

pub fn otp_digest(pepper: &[u8], challenge_id: Uuid, email: &str, code: &str) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"solana-tax-protocol/email-otp/v1\0");
    digest.update(pepper);
    digest.update(challenge_id.as_bytes());
    digest.update(email.as_bytes());
    digest.update(code.as_bytes());
    digest.finalize().into()
}

pub fn constant_time_digest_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.ct_eq(right).into()
}

pub fn derive_role_scopes(
    base_scopes: &[String],
    administrator_scopes: &[String],
    moderator_scopes: &[String],
    roles: &[String],
) -> Vec<String> {
    let mut scopes =
        Vec::with_capacity(base_scopes.len() + administrator_scopes.len() + moderator_scopes.len());
    for scope in base_scopes {
        if !scopes.iter().any(|existing| existing == scope) {
            scopes.push(scope.clone());
        }
    }
    if roles.iter().any(|role| role == "administrator") {
        for scope in administrator_scopes {
            if !scopes.iter().any(|existing| existing == scope) {
                scopes.push(scope.clone());
            }
        }
    }
    if roles.iter().any(|role| role == "moderator") {
        for scope in moderator_scopes {
            if !scopes.iter().any(|existing| existing == scope) {
                scopes.push(scope.clone());
            }
        }
    }
    scopes
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleScopeProfile {
    pub role: String,
    pub scopes: Vec<String>,
}

pub fn derive_role_scopes_with_profiles(
    base_scopes: &[String],
    administrator_scopes: &[String],
    moderator_scopes: &[String],
    roles: &[String],
    profiles: &[RoleScopeProfile],
) -> Vec<String> {
    let mut scopes = derive_role_scopes(base_scopes, administrator_scopes, moderator_scopes, roles);
    for role in roles {
        if let Some(profile) = profiles.iter().find(|profile| profile.role == *role) {
            for scope in &profile.scopes {
                if !scopes.iter().any(|existing| existing == scope) {
                    scopes.push(scope.clone());
                }
            }
        }
    }
    scopes
}

#[derive(Clone, Debug)]
pub struct TokenIssuer {
    issuer: Arc<str>,
    audience: Arc<str>,
    key_id: Arc<str>,
    signing_key: SigningKey,
}

#[derive(Clone, Debug)]
pub struct TokenRequest {
    pub subject: Uuid,
    pub merchant_id: Uuid,
    pub email: String,
    pub scopes: Vec<String>,
    pub roles: Vec<String>,
    pub now_unix: i64,
    pub nonce: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub id_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedAccessToken {
    pub subject: Uuid,
    pub merchant_id: Uuid,
    pub email: String,
    pub expires_at_unix: i64,
    pub scopes: Vec<String>,
    pub roles: Vec<String>,
}

#[derive(Serialize)]
struct JwtHeader<'a> {
    alg: &'static str,
    kid: &'a str,
    typ: &'static str,
}

#[derive(Deserialize)]
struct JwtHeaderOwned {
    alg: String,
    kid: String,
    typ: String,
}

#[derive(Deserialize)]
struct AccessClaimsOwned {
    iss: String,
    sub: String,
    aud: String,
    exp: i64,
    iat: i64,
    nbf: i64,
    merchant_id: String,
    email: String,
    scope: String,
    roles: Vec<String>,
}

#[derive(Serialize)]
struct AccessClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    exp: i64,
    iat: i64,
    nbf: i64,
    merchant_id: &'a str,
    email: &'a str,
    scope: &'a str,
    roles: &'a [String],
}

#[derive(Serialize)]
struct IdClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    exp: i64,
    iat: i64,
    nbf: i64,
    nonce: Option<&'a str>,
    email: &'a str,
    email_verified: bool,
}

impl TokenIssuer {
    pub fn new(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        key_id: impl Into<String>,
        signing_key: SigningKey,
    ) -> Result<Self, AuthDomainError> {
        let issuer = issuer.into();
        let audience = audience.into();
        let key_id = key_id.into();
        if issuer.trim().is_empty()
            || issuer.len() > 2_048
            || audience.trim().is_empty()
            || audience.len() > MAX_TOKEN_CLAIM_BYTES
            || key_id.trim().is_empty()
            || key_id.len() > MAX_TOKEN_CLAIM_BYTES
        {
            return Err(AuthDomainError::InvalidConfiguration(
                "issuer, audience, and key ID must be bounded and non-empty".to_string(),
            ));
        }
        Ok(Self {
            issuer: Arc::from(issuer),
            audience: Arc::from(audience),
            key_id: Arc::from(key_id),
            signing_key,
        })
    }

    pub fn issue_tokens(&self, request: TokenRequest) -> Result<TokenResponse, AuthDomainError> {
        let email = normalize_email(&request.email)?;
        if request.now_unix <= 0
            || request.scopes.len() > 32
            || request.roles.len() > 32
            || request
                .scopes
                .iter()
                .any(|value| value.is_empty() || value.len() > MAX_TOKEN_CLAIM_BYTES)
            || request
                .roles
                .iter()
                .any(|value| value.is_empty() || value.len() > MAX_TOKEN_CLAIM_BYTES)
        {
            return Err(AuthDomainError::InvalidConfiguration(
                "token request contains an invalid bounded claim".to_string(),
            ));
        }
        let subject = request.subject.to_string();
        let merchant_id = request.merchant_id.to_string();
        let scope = request.scopes.join(" ");
        let expires_at = request
            .now_unix
            .checked_add(TOKEN_TTL_SECONDS)
            .ok_or_else(|| {
                AuthDomainError::InvalidConfiguration("token expiry overflow".to_string())
            })?;
        let access_claims = AccessClaims {
            iss: &self.issuer,
            sub: &subject,
            aud: &self.audience,
            exp: expires_at,
            iat: request.now_unix,
            nbf: request.now_unix,
            merchant_id: &merchant_id,
            email: &email,
            scope: &scope,
            roles: &request.roles,
        };
        let id_claims = IdClaims {
            iss: &self.issuer,
            sub: &subject,
            aud: &self.audience,
            exp: expires_at,
            iat: request.now_unix,
            nbf: request.now_unix,
            nonce: request.nonce.as_deref(),
            email: &email,
            email_verified: true,
        };
        Ok(TokenResponse {
            access_token: self.sign_claims(&access_claims)?,
            id_token: self.sign_claims(&id_claims)?,
            token_type: "Bearer".to_string(),
            expires_in: TOKEN_TTL_SECONDS,
            scope,
        })
    }

    pub fn jwks_json(&self) -> Result<String, AuthDomainError> {
        serde_json::to_string(&serde_json::json!({
            "keys": [{
                "kty": "OKP",
                "crv": "Ed25519",
                "alg": "EdDSA",
                "use": "sig",
                "kid": self.key_id.as_ref(),
                "x": URL_SAFE_NO_PAD.encode(self.signing_key.verifying_key().to_bytes()),
            }]
        }))
        .map_err(|_| AuthDomainError::Serialization)
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn verify_access_token(
        &self,
        token: &str,
        now_unix: i64,
    ) -> Result<VerifiedAccessToken, AuthDomainError> {
        let mut segments = token.split('.');
        let encoded_header = segments.next().ok_or(AuthDomainError::InvalidToken)?;
        let encoded_payload = segments.next().ok_or(AuthDomainError::InvalidToken)?;
        let encoded_signature = segments.next().ok_or(AuthDomainError::InvalidToken)?;
        if segments.next().is_some() || token.len() > 8 * 1024 || now_unix <= 0 {
            return Err(AuthDomainError::InvalidToken);
        }
        let header_bytes = URL_SAFE_NO_PAD
            .decode(encoded_header.as_bytes())
            .map_err(|_| AuthDomainError::InvalidToken)?;
        let header: JwtHeaderOwned =
            serde_json::from_slice(&header_bytes).map_err(|_| AuthDomainError::InvalidToken)?;
        if header.alg != "EdDSA" || header.typ != "JWT" || header.kid != self.key_id.as_ref() {
            return Err(AuthDomainError::InvalidToken);
        }
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(encoded_payload.as_bytes())
            .map_err(|_| AuthDomainError::InvalidToken)?;
        let claims: AccessClaimsOwned =
            serde_json::from_slice(&payload_bytes).map_err(|_| AuthDomainError::InvalidToken)?;
        if claims.iss != self.issuer.as_ref()
            || claims.aud != self.audience.as_ref()
            || claims.exp <= now_unix
            || claims.nbf > now_unix
            || claims.iat > now_unix.saturating_add(30)
            || claims.roles.len() > 32
            || claims.scopes_len_exceeds_limit()
        {
            return Err(AuthDomainError::InvalidToken);
        }
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(encoded_signature.as_bytes())
            .map_err(|_| AuthDomainError::InvalidToken)?;
        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| AuthDomainError::InvalidToken)?;
        let signing_input = format!("{encoded_header}.{encoded_payload}");
        self.signing_key
            .verifying_key()
            .verify(signing_input.as_bytes(), &signature)
            .map_err(|_| AuthDomainError::InvalidToken)?;
        let subject = Uuid::parse_str(&claims.sub).map_err(|_| AuthDomainError::InvalidToken)?;
        let merchant_id =
            Uuid::parse_str(&claims.merchant_id).map_err(|_| AuthDomainError::InvalidToken)?;
        let email = normalize_email(&claims.email).map_err(|_| AuthDomainError::InvalidToken)?;
        if claims
            .roles
            .iter()
            .any(|role| role.is_empty() || role.len() > MAX_TOKEN_CLAIM_BYTES)
            || claims
                .scope
                .split_whitespace()
                .any(|scope| scope.is_empty() || scope.len() > MAX_TOKEN_CLAIM_BYTES)
        {
            return Err(AuthDomainError::InvalidToken);
        }
        Ok(VerifiedAccessToken {
            subject,
            merchant_id,
            email,
            expires_at_unix: claims.exp,
            scopes: claims
                .scope
                .split_whitespace()
                .map(str::to_string)
                .collect(),
            roles: claims.roles,
        })
    }

    fn sign_claims<T: Serialize>(&self, claims: &T) -> Result<String, AuthDomainError> {
        let header = serde_json::to_vec(&JwtHeader {
            alg: "EdDSA",
            kid: &self.key_id,
            typ: "JWT",
        })
        .map_err(|_| AuthDomainError::Serialization)?;
        let payload = serde_json::to_vec(claims).map_err(|_| AuthDomainError::Serialization)?;
        let encoded_header = URL_SAFE_NO_PAD.encode(header);
        let encoded_payload = URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{encoded_header}.{encoded_payload}");
        let signature = self.signing_key.sign(signing_input.as_bytes());
        Ok(format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }
}

trait AccessClaimsBounds {
    fn scopes_len_exceeds_limit(&self) -> bool;
}

impl AccessClaimsBounds for AccessClaimsOwned {
    fn scopes_len_exceeds_limit(&self) -> bool {
        self.scope.len() > MAX_TOKEN_CLAIM_BYTES * 32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use uuid::Uuid;

    #[test]
    fn normalizes_email_without_accepting_ambiguous_input() {
        assert_eq!(
            normalize_email("  Alice.Example+test@Example.COM ").unwrap(),
            "alice.example+test@example.com"
        );
        assert!(normalize_email("alice@example.com\nBcc: attacker@example.com").is_err());
        assert!(normalize_email("alice@example").is_err());
        assert!(normalize_email(" ").is_err());
    }

    #[test]
    fn otp_digest_is_bound_to_challenge_and_email() {
        let challenge = Uuid::from_u128(1);
        let first = otp_digest(b"pepper", challenge, "alice@example.com", "123456");
        assert_eq!(
            first,
            otp_digest(b"pepper", challenge, "alice@example.com", "123456")
        );
        assert_ne!(
            first,
            otp_digest(b"pepper", Uuid::from_u128(2), "alice@example.com", "123456")
        );
        assert_ne!(
            first,
            otp_digest(b"pepper", challenge, "bob@example.com", "123456")
        );
        assert_ne!(
            first,
            otp_digest(b"other", challenge, "alice@example.com", "123456")
        );
        assert!(!constant_time_digest_eq(
            &first,
            &otp_digest(b"pepper", challenge, "alice@example.com", "123457")
        ));
    }

    #[test]
    fn otp_policy_is_explicit_and_bounded() {
        let policy = OtpPolicy::default();
        assert_eq!(policy.ttl_seconds, 300);
        assert_eq!(policy.resend_cooldown_seconds, 60);
        assert_eq!(policy.rate_limit_window_seconds, 900);
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.max_email_requests, 3);
        assert_eq!(policy.max_source_requests, 20);
        assert!(validate_otp_code("123456").is_ok());
        assert!(validate_otp_code("12345").is_err());
        assert!(validate_otp_code("12345a").is_err());
    }

    #[test]
    fn authentication_intent_is_explicit_and_backward_compatible() {
        assert_eq!(parse_auth_intent(None), Ok(AuthIntent::SignIn));
        assert_eq!(parse_auth_intent(Some("signin")), Ok(AuthIntent::SignIn));
        assert_eq!(parse_auth_intent(Some("signup")), Ok(AuthIntent::SignUp));
        assert_eq!(
            parse_auth_intent(Some("administrator")),
            Err(AuthDomainError::InvalidIntent)
        );
    }

    #[test]
    fn generated_otp_is_six_decimal_digits() {
        let code = generate_otp().unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
    }

    #[test]
    fn role_scope_profiles_are_bounded_and_deterministic() {
        let base = vec!["tax:quote".to_string(), "audit:read".to_string()];
        let administrator = vec!["audit:read".to_string(), "identity:role:manage".to_string()];
        let moderator = vec!["audit:read".to_string(), "tax:moderate".to_string()];
        assert_eq!(
            derive_role_scopes(
                &base,
                &administrator,
                &moderator,
                &["merchant_operator".to_string()]
            ),
            vec!["tax:quote", "audit:read"]
        );
        assert_eq!(
            derive_role_scopes(
                &base,
                &administrator,
                &moderator,
                &["moderator".to_string()]
            ),
            vec!["tax:quote", "audit:read", "tax:moderate"]
        );
        assert_eq!(
            derive_role_scopes(
                &base,
                &administrator,
                &moderator,
                &["administrator".to_string(), "moderator".to_string()]
            ),
            vec![
                "tax:quote",
                "audit:read",
                "identity:role:manage",
                "tax:moderate"
            ]
        );
        assert_eq!(
            derive_role_scopes(&base, &administrator, &moderator, &["unknown".to_string()]),
            vec!["tax:quote", "audit:read"]
        );
    }

    #[test]
    fn explicit_domain_roles_add_only_their_catalog_scopes() {
        let profiles = vec![
            RoleScopeProfile {
                role: "tax_rule_author".to_string(),
                scopes: vec!["tax:rule:read".to_string()],
            },
            RoleScopeProfile {
                role: "vault_release_proposer".to_string(),
                scopes: vec!["tax:vault:propose".to_string()],
            },
        ];
        let scopes = derive_role_scopes_with_profiles(
            &["tax:quote".to_string()],
            &["identity:role:manage".to_string()],
            &["audit:read".to_string()],
            &["tax_rule_author".to_string(), "unknown".to_string()],
            &profiles,
        );
        assert_eq!(scopes, vec!["tax:quote", "tax:rule:read"]);
        assert!(!scopes.iter().any(|scope| scope == "tax:vault:propose"));
        assert!(!scopes.iter().any(|scope| scope == "identity:role:manage"));
    }

    #[test]
    fn jwt_tokens_match_api_claim_contract_and_jwks_is_public_only() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let issuer = TokenIssuer::new(
            "http://localhost:18085",
            "tax-api",
            "auth-local-v1",
            signing_key,
        )
        .unwrap();
        let subject = Uuid::from_u128(1);
        let merchant = Uuid::from_u128(2);
        let response = issuer
            .issue_tokens(TokenRequest {
                subject,
                merchant_id: merchant,
                email: "alice@example.com".to_string(),
                scopes: vec!["tax:quote".to_string()],
                roles: vec!["merchant_operator".to_string()],
                now_unix: 1_800_000_000,
                nonce: Some("nonce-1".to_string()),
            })
            .unwrap();
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 900);
        assert_eq!(response.access_token.split('.').count(), 3);
        assert_eq!(response.id_token.split('.').count(), 3);
        let verified = issuer
            .verify_access_token(&response.access_token, 1_800_000_001)
            .unwrap();
        assert_eq!(verified.subject, subject);
        assert_eq!(verified.merchant_id, merchant);
        assert_eq!(verified.email, "alice@example.com");
        assert_eq!(verified.roles, vec!["merchant_operator"]);
        assert_eq!(verified.scopes, vec!["tax:quote"]);
        let mut tampered = response.access_token.clone();
        tampered.push('x');
        assert_eq!(
            issuer.verify_access_token(&tampered, 1_800_000_001),
            Err(AuthDomainError::InvalidToken)
        );
        let jwks = issuer.jwks_json().unwrap();
        assert!(jwks.contains("\"kid\":\"auth-local-v1\""));
        assert!(!jwks.contains("private"));
        assert!(!jwks.contains("seed"));
    }

    #[test]
    fn token_configuration_rejects_unbounded_values() {
        assert!(TokenIssuer::new("", "tax-api", "kid", SigningKey::from_bytes(&[7; 32])).is_err());
        assert!(
            TokenIssuer::new(
                "https://issuer.example",
                "",
                "kid",
                SigningKey::from_bytes(&[7; 32])
            )
            .is_err()
        );
        assert!(
            TokenIssuer::new(
                "https://issuer.example",
                "tax-api",
                "",
                SigningKey::from_bytes(&[7; 32])
            )
            .is_err()
        );
    }
}
