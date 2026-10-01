use arc_swap::ArcSwap;
use axum::http::HeaderValue;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read, path::Path, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

const MAX_AUTHORIZATION_BYTES: usize = 8 * 1024;
const MAX_JWKS_KEYS: usize = 32;
pub const MAX_JWKS_DOCUMENT_BYTES: usize = 64 * 1024;
const MAX_TOKEN_LIFETIME_SECONDS: i64 = 3_600;
const CLOCK_SKEW_SECONDS: i64 = 30;
const MAX_SCOPES: usize = 32;
const MAX_ROLES: usize = 32;
const MAX_CLAIM_VALUE_BYTES: usize = 128;

#[derive(Clone, Debug)]
pub struct JwtAuthenticator {
    issuer: Arc<str>,
    audience: Arc<str>,
    keys: Arc<ArcSwap<HashMap<String, VerifyingKey>>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedPrincipal {
    pub subject: String,
    pub merchant_id: Uuid,
    pub expires_at_unix: i64,
    pub scopes: Vec<String>,
    pub roles: Vec<String>,
}

impl AuthenticatedPrincipal {
    pub fn has_scope(&self, required: &str) -> bool {
        !required.is_empty() && self.scopes.iter().any(|scope| scope == required)
    }

    pub fn require_scope(&self, required: &str) -> Result<(), AuthError> {
        if self.has_scope(required) {
            Ok(())
        } else {
            Err(AuthError::InsufficientScope)
        }
    }

    pub fn has_role(&self, required: &str) -> bool {
        !required.is_empty() && self.roles.iter().any(|role| role == required)
    }

    pub fn require_role(&self, required: &str) -> Result<(), AuthError> {
        if self.has_role(required) {
            Ok(())
        } else {
            Err(AuthError::InsufficientRole)
        }
    }

    pub fn require_any_role(&self, required: &[&str]) -> Result<(), AuthError> {
        if required.iter().any(|role| self.has_role(role)) {
            Ok(())
        } else {
            Err(AuthError::InsufficientRole)
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("authentication is not configured")]
    NotConfigured,
    #[error("authentication clock is unavailable")]
    ClockUnavailable,
    #[error("authorization header is missing")]
    MissingHeader,
    #[error("authorization header is invalid")]
    InvalidHeader,
    #[error("bearer token is too large")]
    TokenTooLarge,
    #[error("bearer token is malformed")]
    MalformedToken,
    #[error("authentication token is invalid")]
    InvalidToken,
    #[error("authentication token is expired")]
    ExpiredToken,
    #[error("authentication token is not active")]
    NotYetValid,
    #[error("authentication key is unknown")]
    UnknownKey,
    #[error("authenticated principal lacks the required scope")]
    InsufficientScope,
    #[error("authenticated principal lacks the required role")]
    InsufficientRole,
    #[error("authentication configuration is invalid: {0}")]
    InvalidConfiguration(String),
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured | Self::ClockUnavailable => "AUTHENTICATION_UNAVAILABLE",
            Self::MissingHeader
            | Self::InvalidHeader
            | Self::TokenTooLarge
            | Self::MalformedToken
            | Self::InvalidToken
            | Self::ExpiredToken
            | Self::NotYetValid
            | Self::UnknownKey => "AUTHENTICATION_FAILED",
            Self::InsufficientScope => "INSUFFICIENT_SCOPE",
            Self::InsufficientRole => "INSUFFICIENT_ROLE",
            Self::InvalidConfiguration(_) => "AUTHENTICATION_UNAVAILABLE",
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::NotConfigured | Self::ClockUnavailable | Self::InvalidConfiguration(_) => {
                "authentication is temporarily unavailable"
            }
            Self::InsufficientScope | Self::InsufficientRole => "authorization failed",
            _ => "authentication failed",
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonWebKeySet {
    keys: Vec<JsonWebKey>,
}

#[derive(Debug, Deserialize)]
struct JsonWebKey {
    kid: String,
    kty: String,
    crv: String,
    alg: String,
    x: String,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
    typ: String,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    aud: Audience,
    exp: i64,
    iat: i64,
    nbf: Option<i64>,
    merchant_id: String,
    scope: Option<String>,
    roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl JwtAuthenticator {
    pub fn from_jwks_json(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        jwks_json: &str,
    ) -> Result<Self, AuthError> {
        let issuer = issuer.into();
        let audience = audience.into();
        if issuer.trim().is_empty() || audience.trim().is_empty() {
            return Err(AuthError::InvalidConfiguration(
                "issuer and audience are required".to_string(),
            ));
        }
        let keys = parse_jwks(jwks_json)?;
        Ok(Self {
            issuer: Arc::from(issuer),
            audience: Arc::from(audience),
            keys: Arc::new(ArcSwap::from_pointee(keys)),
        })
    }

    pub fn from_jwks_file(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        path: &Path,
    ) -> Result<Self, AuthError> {
        let document = read_jwks_file(path)?;
        Self::from_jwks_json(issuer, audience, &document)
    }

    pub fn reload_from_jwks_file(&self, path: &Path) -> Result<usize, AuthError> {
        let document = read_jwks_file(path)?;
        let keys = parse_jwks(&document)?;
        let key_count = keys.len();
        self.keys.store(Arc::new(keys));
        Ok(key_count)
    }

    pub fn key_count(&self) -> usize {
        self.keys.load().len()
    }
}

fn read_jwks_file(path: &Path) -> Result<String, AuthError> {
    let file = File::open(path).map_err(|_| {
        AuthError::InvalidConfiguration("JWKS file could not be opened".to_string())
    })?;
    let mut bytes = Vec::new();
    file.take((MAX_JWKS_DOCUMENT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| AuthError::InvalidConfiguration("JWKS file could not be read".to_string()))?;
    if bytes.len() > MAX_JWKS_DOCUMENT_BYTES {
        return Err(AuthError::InvalidConfiguration(
            "JWKS file exceeds the bounded document size".to_string(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| AuthError::InvalidConfiguration("JWKS file is not UTF-8".to_string()))
}

fn parse_jwks(jwks_json: &str) -> Result<HashMap<String, VerifyingKey>, AuthError> {
    if jwks_json.len() > MAX_JWKS_DOCUMENT_BYTES {
        return Err(AuthError::InvalidConfiguration(
            "JWKS document exceeds the bounded document size".to_string(),
        ));
    }
    let jwks: JsonWebKeySet = serde_json::from_str(jwks_json).map_err(|error| {
        AuthError::InvalidConfiguration(format!("JWKS JSON is invalid: {error}"))
    })?;
    if jwks.keys.is_empty() || jwks.keys.len() > MAX_JWKS_KEYS {
        return Err(AuthError::InvalidConfiguration(
            "JWKS must contain between 1 and 32 keys".to_string(),
        ));
    }
    let mut keys = HashMap::with_capacity(jwks.keys.len());
    for key in jwks.keys {
        if key.kid.trim().is_empty()
            || key.kty != "OKP"
            || key.crv != "Ed25519"
            || key.alg != "EdDSA"
        {
            return Err(AuthError::InvalidConfiguration(
                "JWKS contains an unsupported Ed25519 key".to_string(),
            ));
        }
        let public_key_bytes = URL_SAFE_NO_PAD.decode(key.x.as_bytes()).map_err(|_| {
            AuthError::InvalidConfiguration("JWKS public key is not base64url".to_string())
        })?;
        let public_key: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
            AuthError::InvalidConfiguration("JWKS public key must be 32 bytes".to_string())
        })?;
        if keys
            .insert(
                key.kid,
                VerifyingKey::from_bytes(&public_key).map_err(|_| {
                    AuthError::InvalidConfiguration("JWKS public key is invalid".to_string())
                })?,
            )
            .is_some()
        {
            return Err(AuthError::InvalidConfiguration(
                "JWKS contains duplicate key identifiers".to_string(),
            ));
        }
    }
    Ok(keys)
}

impl JwtAuthenticator {
    pub fn authenticate(
        &self,
        authorization: Option<&HeaderValue>,
        now_unix: i64,
    ) -> Result<AuthenticatedPrincipal, AuthError> {
        let header = authorization.ok_or(AuthError::MissingHeader)?;
        let header = header.to_str().map_err(|_| AuthError::InvalidHeader)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidHeader)?;
        if token.is_empty() {
            return Err(AuthError::InvalidHeader);
        }
        if token.len() > MAX_AUTHORIZATION_BYTES {
            return Err(AuthError::TokenTooLarge);
        }
        let mut sections = token.split('.');
        let header_segment = sections.next().ok_or(AuthError::MalformedToken)?;
        let claims_segment = sections.next().ok_or(AuthError::MalformedToken)?;
        let signature_segment = sections.next().ok_or(AuthError::MalformedToken)?;
        if sections.next().is_some()
            || header_segment.is_empty()
            || claims_segment.is_empty()
            || signature_segment.is_empty()
        {
            return Err(AuthError::MalformedToken);
        }
        let header_bytes = URL_SAFE_NO_PAD
            .decode(header_segment.as_bytes())
            .map_err(|_| AuthError::MalformedToken)?;
        let jwt_header: JwtHeader =
            serde_json::from_slice(&header_bytes).map_err(|_| AuthError::MalformedToken)?;
        if jwt_header.typ != "JWT" || jwt_header.alg != "EdDSA" || jwt_header.kid.is_empty() {
            return Err(AuthError::InvalidToken);
        }
        let keys = self.keys.load();
        let verifying_key = keys.get(&jwt_header.kid).ok_or(AuthError::UnknownKey)?;
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(signature_segment.as_bytes())
            .map_err(|_| AuthError::MalformedToken)?;
        let signature =
            Signature::from_slice(&signature_bytes).map_err(|_| AuthError::InvalidToken)?;
        let signing_input = format!("{header_segment}.{claims_segment}");
        verifying_key
            .verify(signing_input.as_bytes(), &signature)
            .map_err(|_| AuthError::InvalidToken)?;

        let claims_bytes = URL_SAFE_NO_PAD
            .decode(claims_segment.as_bytes())
            .map_err(|_| AuthError::MalformedToken)?;
        let claims: JwtClaims =
            serde_json::from_slice(&claims_bytes).map_err(|_| AuthError::InvalidToken)?;
        if claims.iss != self.issuer.as_ref()
            || !claims.aud.matches(self.audience.as_ref())
            || claims.sub.trim().is_empty()
            || claims.sub.len() > 256
        {
            return Err(AuthError::InvalidToken);
        }
        if claims.exp <= claims.iat
            || claims.exp.saturating_sub(claims.iat) > MAX_TOKEN_LIFETIME_SECONDS
        {
            return Err(AuthError::InvalidToken);
        }
        if now_unix > claims.exp.saturating_add(CLOCK_SKEW_SECONDS) {
            return Err(AuthError::ExpiredToken);
        }
        if claims.iat > now_unix.saturating_add(CLOCK_SKEW_SECONDS)
            || claims
                .nbf
                .is_some_and(|not_before| not_before > now_unix.saturating_add(CLOCK_SKEW_SECONDS))
        {
            return Err(AuthError::NotYetValid);
        }
        let merchant_id =
            Uuid::parse_str(&claims.merchant_id).map_err(|_| AuthError::InvalidToken)?;
        let scopes = parse_scopes(claims.scope.as_deref())?;
        let roles = parse_roles(claims.roles.as_deref())?;
        Ok(AuthenticatedPrincipal {
            subject: claims.sub,
            merchant_id,
            expires_at_unix: claims.exp,
            scopes,
            roles,
        })
    }
}

fn parse_scopes(scope: Option<&str>) -> Result<Vec<String>, AuthError> {
    let Some(scope) = scope else {
        return Ok(Vec::new());
    };
    let values = scope.split_whitespace().collect::<Vec<_>>();
    if values.len() > MAX_SCOPES
        || values
            .iter()
            .any(|value| value.is_empty() || value.len() > MAX_CLAIM_VALUE_BYTES)
    {
        return Err(AuthError::InvalidToken);
    }
    Ok(values.into_iter().map(str::to_owned).collect())
}

fn parse_roles(roles: Option<&[String]>) -> Result<Vec<String>, AuthError> {
    let Some(roles) = roles else {
        return Ok(Vec::new());
    };
    if roles.len() > MAX_ROLES
        || roles
            .iter()
            .any(|role| role.trim().is_empty() || role.len() > MAX_CLAIM_VALUE_BYTES)
    {
        return Err(AuthError::InvalidToken);
    }
    Ok(roles.to_vec())
}

impl Audience {
    fn matches(&self, expected: &str) -> bool {
        match self {
            Self::One(value) => value == expected,
            Self::Many(values) => values.iter().any(|value| value == expected),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_authenticator() -> JwtAuthenticator {
    let public_key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key();
    let jwks = serde_json::json!({
        "keys": [{
            "kid": "test-key",
            "kty": "OKP",
            "crv": "Ed25519",
            "alg": "EdDSA",
            "x": URL_SAFE_NO_PAD.encode(public_key.to_bytes())
        }]
    });
    JwtAuthenticator::from_jwks_json("https://issuer.test", "tax-api", &jwks.to_string())
        .expect("test authenticator is valid")
}

#[cfg(test)]
pub(crate) fn test_token(now_unix: i64) -> (String, Uuid) {
    test_token_with_scope(now_unix, "tax:quote")
}

#[cfg(test)]
pub(crate) fn test_token_with_scope(now_unix: i64, scope: &str) -> (String, Uuid) {
    test_token_with_scope_and_roles(now_unix, scope, "test-user", &[])
}

#[cfg(test)]
pub(crate) fn test_token_with_scope_and_roles(
    now_unix: i64,
    scope: &str,
    subject: &str,
    roles: &[&str],
) -> (String, Uuid) {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let merchant_id = Uuid::from_u128(0x1234567890abcdef1234567890abcdef);
    let header = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "alg": "EdDSA", "kid": "test-key", "typ": "JWT"
        }))
        .expect("test header"),
    );
    let claims = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "iss": "https://issuer.test",
            "sub": subject,
            "aud": "tax-api",
            "exp": now_unix + 300,
            "iat": now_unix - 100,
            "nbf": now_unix - 100,
            "merchant_id": merchant_id.to_string(),
            "scope": scope,
            "roles": roles
        }))
        .expect("test claims"),
    );
    let signing_input = format!("{header}.{claims}");
    use ed25519_dalek::Signer;
    let signature = signing_key.sign(signing_input.as_bytes());
    (
        format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ),
        merchant_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use ed25519_dalek::Signer;
    use std::fs;

    #[test]
    fn accepts_string_and_array_audience() {
        let authenticator = test_authenticator();
        let (token, merchant_id) = test_token(1_800_000_000);
        let principal = authenticator
            .authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {token}")).unwrap()),
                1_800_000_100,
            )
            .unwrap();
        assert_eq!(principal.merchant_id, merchant_id);
    }

    #[test]
    fn rejects_tampered_and_expired_tokens() {
        let authenticator = test_authenticator();
        let (token, _) = test_token(1_800_000_000);
        let mut parts = token.split('.');
        let header = parts.next().unwrap();
        let claims = parts.next().unwrap();
        let tampered = format!("{header}.{claims}.{}", URL_SAFE_NO_PAD.encode([0; 64]));
        assert!(matches!(
            authenticator.authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {tampered}")).unwrap()),
                1_800_000_100,
            ),
            Err(AuthError::InvalidToken)
        ));
        let (expired, _) = test_token(1_799_000_000);
        assert!(matches!(
            authenticator.authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {expired}")).unwrap()),
                1_800_000_000,
            ),
            Err(AuthError::ExpiredToken)
        ));
    }

    #[test]
    fn valid_file_rotation_replaces_snapshot_and_invalid_reload_retains_it() {
        let first_key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
        let second_key = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
        let path = std::env::temp_dir().join(format!(
            "solana-tax-api-jwks-{}-{}.json",
            std::process::id(),
            Uuid::new_v4()
        ));
        let first_document = jwks_document("first-key", &first_key);
        let second_document = jwks_document("second-key", &second_key);
        fs::write(&path, &first_document).expect("first JWKS fixture");

        let authenticator =
            JwtAuthenticator::from_jwks_file("https://issuer.test", "tax-api", &path)
                .expect("file-backed authenticator");
        let first_token = token_for_key("first-key", &first_key, 1_800_000_000);
        let second_token = token_for_key("second-key", &second_key, 1_800_000_000);
        assert!(
            authenticator
                .authenticate(
                    Some(&HeaderValue::from_str(&format!("Bearer {first_token}")).unwrap()),
                    1_800_000_100,
                )
                .is_ok()
        );

        fs::write(&path, &second_document).expect("second JWKS fixture");
        assert!(matches!(authenticator.reload_from_jwks_file(&path), Ok(1)));
        assert!(matches!(
            authenticator.authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {first_token}")).unwrap()),
                1_800_000_100,
            ),
            Err(AuthError::UnknownKey)
        ));
        assert!(
            authenticator
                .authenticate(
                    Some(&HeaderValue::from_str(&format!("Bearer {second_token}")).unwrap()),
                    1_800_000_100,
                )
                .is_ok()
        );

        fs::write(&path, "{not-json").expect("invalid JWKS fixture");
        assert!(matches!(
            authenticator.reload_from_jwks_file(&path),
            Err(AuthError::InvalidConfiguration(_))
        ));
        assert!(
            authenticator
                .authenticate(
                    Some(&HeaderValue::from_str(&format!("Bearer {second_token}")).unwrap()),
                    1_800_000_100,
                )
                .is_ok()
        );
        let _ = fs::remove_file(path);
    }

    fn jwks_document(kid: &str, signing_key: &ed25519_dalek::SigningKey) -> String {
        serde_json::json!({
            "keys": [{
                "kid": kid,
                "kty": "OKP",
                "crv": "Ed25519",
                "alg": "EdDSA",
                "x": URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes())
            }]
        })
        .to_string()
    }

    fn token_for_key(kid: &str, signing_key: &ed25519_dalek::SigningKey, now_unix: i64) -> String {
        let header = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "alg": "EdDSA", "kid": kid, "typ": "JWT"
            }))
            .expect("test header"),
        );
        let claims = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "iss": "https://issuer.test",
                "sub": "rotation-user",
                "aud": "tax-api",
                "exp": now_unix + 300,
                "iat": now_unix - 100,
                "nbf": now_unix - 100,
                "merchant_id": "12345678-1234-5678-1234-567812345678",
                "scope": "tax:quote",
                "roles": []
            }))
            .expect("test claims"),
        );
        let signing_input = format!("{header}.{claims}");
        format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signing_key.sign(signing_input.as_bytes()).to_bytes())
        )
    }

    #[test]
    fn exposes_bounded_scopes_and_requires_exact_authorization() {
        let authenticator = test_authenticator();
        let (token, _) = test_token_with_scope(1_800_000_000, "tax:quote audit:read");
        let principal = authenticator
            .authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {token}")).unwrap()),
                1_800_000_100,
            )
            .unwrap();
        assert!(principal.has_scope("tax:quote"));
        assert!(!principal.has_scope("tax"));
        assert!(principal.require_scope("tax:quote").is_ok());
        assert!(matches!(
            principal.require_scope("settlement:submit"),
            Err(AuthError::InsufficientScope)
        ));
    }

    #[test]
    fn requires_exact_governance_role() {
        let authenticator = test_authenticator();
        let (token, _) = test_token_with_scope_and_roles(
            1_800_000_000,
            "tax:rule:review",
            "reviewer",
            &["tax_rule_reviewer"],
        );
        let principal = authenticator
            .authenticate(
                Some(&HeaderValue::from_str(&format!("Bearer {token}")).unwrap()),
                1_800_000_100,
            )
            .unwrap();
        assert!(principal.require_role("tax_rule_reviewer").is_ok());
        assert!(
            principal
                .require_any_role(&["tax_rule_author", "tax_rule_reviewer"])
                .is_ok()
        );
        assert!(matches!(
            principal.require_any_role(&["tax_rule_approver", "tax_rule_auditor"]),
            Err(AuthError::InsufficientRole)
        ));
        assert!(matches!(
            principal.require_role("tax_rule_approver"),
            Err(AuthError::InsufficientRole)
        ));
    }
}
