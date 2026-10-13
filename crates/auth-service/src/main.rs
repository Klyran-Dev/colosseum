#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use async_trait::async_trait;
use auth_service::{
    AuthIntent, OtpPolicy, RoleScopeProfile, TokenIssuer, TokenRequest, VerifiedAccessToken,
    constant_time_digest_eq, derive_role_scopes_with_profiles, generate_otp, normalize_email,
    otp_digest, parse_auth_intent, validate_otp_code,
};
use axum::{
    Router,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use deadpool_postgres::{Config as PoolConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use ed25519_dalek::SigningKey;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart},
    transport::smtp::authentication::Credentials,
};
use reqwest::{Url, header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    env, fs,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::sync::Semaphore;
use tokio_postgres::NoTls;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use tracing::{error, info, warn};
use uuid::Uuid;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8085";
const DEFAULT_DATABASE_URL: &str =
    "postgres://tax_protocol:local-development-only@postgres:5432/tax_protocol";
const DEFAULT_ISSUER: &str = "http://localhost:18085";
const DEFAULT_AUDIENCE: &str = "tax-api";
const DEFAULT_LOCAL_INBOX: &str = "/var/lib/auth/inbox";
const DEFAULT_PROVIDER_ENDPOINT: &str = "https://api.resend.com/emails";
const DEFAULT_LOCAL_APP_BASE_URL: &str = "http://localhost:8088";
const DEFAULT_SMTP_HOST: &str = "smtp-relay.gmail.com";
const DEFAULT_SMTP_PORT: &str = "587";
const DEFAULT_SMTP_SECURITY: &str = "starttls";
const DEFAULT_SMTP_AUTHENTICATION: &str = "app-password";
const DEFAULT_SMTP_TIMEOUT_MILLIS: u64 = 10_000;
const MAX_SMTP_TIMEOUT_MILLIS: u64 = 30_000;
const MAX_BODY_BYTES: usize = 8 * 1024;
const MAX_PROVIDER_ENDPOINT_BYTES: usize = 2 * 1024;
const MAX_PROVIDER_API_KEY_BYTES: usize = 4 * 1024;
const MAX_PROVIDER_FROM_BYTES: usize = 320;
const MAX_PROVIDER_RESPONSE_BYTES: usize = 16 * 1024;
const DEFAULT_PROVIDER_TIMEOUT_MILLIS: u64 = 3_000;
const MAX_PROVIDER_TIMEOUT_MILLIS: u64 = 10_000;

#[derive(Clone)]
struct AppState {
    pool: Pool,
    issuer: Arc<TokenIssuer>,
    email_sender: Arc<dyn EmailSender>,
    policy: OtpPolicy,
    otp_pepper: Arc<[u8]>,
    self_signup: bool,
    self_signup_daily_limit: i64,
    default_scopes: Arc<[String]>,
    administrator_scopes: Arc<[String]>,
    moderator_scopes: Arc<[String]>,
    default_roles: Arc<[String]>,
    role_catalog: Arc<[RoleCatalogEntry]>,
    role_profiles: Arc<[RoleScopeProfile]>,
    local_bootstrap_admin_email: Option<Arc<str>>,
    initial_admin_email: Option<Arc<str>>,
    workers: Arc<Semaphore>,
    issuer_url: Arc<str>,
    audience: Arc<str>,
    key_id: Arc<str>,
}

#[derive(Debug, Error)]
enum ServiceError {
    #[error("invalid request")]
    BadRequest,
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("account setup is required")]
    AccountSetupRequired,
    #[error("an account already exists for this email")]
    AccountExists,
    #[error("request rate limited")]
    RateLimited,
    #[error("daily self-signup limit reached")]
    SelfSignupDailyLimitReached,
    #[error("authentication service unavailable")]
    Unavailable,
    #[error("database operation failed")]
    Database(#[source] tokio_postgres::Error),
    #[error("email delivery failed")]
    EmailDelivery,
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("internal service error")]
    Internal,
    #[error("authentication failed")]
    AuthorizationFailed,
    #[error("administrator authorization is required")]
    Forbidden,
    #[error("the role mutation conflicts with an existing request")]
    Conflict,
    #[error("the invitation conflicts with an existing request")]
    InvitationConflict,
    #[error("the requested user was not found")]
    NotFound,
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let next_action = match &self {
            Self::AccountSetupRequired => Some("signup"),
            Self::AccountExists => Some("signin"),
            _ => None,
        };
        let retryable = matches!(
            &self,
            Self::AuthenticationFailed
                | Self::RateLimited
                | Self::SelfSignupDailyLimitReached
                | Self::Unavailable
                | Self::EmailDelivery
        );
        let (status, code, message) = match self {
            Self::BadRequest => (
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST",
                "The request is invalid.",
            ),
            Self::AuthorizationFailed => (
                StatusCode::UNAUTHORIZED,
                "AUTHENTICATION_FAILED",
                "Authentication failed.",
            ),
            Self::AccountSetupRequired => (
                StatusCode::CONFLICT,
                "ACCOUNT_SETUP_REQUIRED",
                "This account has not been set up yet. Choose Create an account to finish setup first.",
            ),
            Self::AccountExists => (
                StatusCode::CONFLICT,
                "ACCOUNT_EXISTS",
                "An account already exists for this email. Start sign-in instead.",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "INSUFFICIENT_ROLE",
                "Administrator authorization is required.",
            ),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "ROLE_MUTATION_CONFLICT",
                "The role mutation conflicts with an existing request.",
            ),
            Self::InvitationConflict => (
                StatusCode::CONFLICT,
                "INVITATION_CONFLICT",
                "The invitation conflicts with an existing request.",
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "USER_NOT_FOUND",
                "The requested user was not found.",
            ),
            Self::AuthenticationFailed => (
                StatusCode::UNAUTHORIZED,
                "AUTHENTICATION_FAILED",
                "The code is invalid or expired.",
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Try again later.",
            ),
            Self::SelfSignupDailyLimitReached => (
                StatusCode::TOO_MANY_REQUESTS,
                "SELF_SIGNUP_DAILY_LIMIT_REACHED",
                "New account creation is temporarily full for today. Please try again tomorrow.",
            ),
            Self::Unavailable | Self::EmailDelivery => (
                StatusCode::SERVICE_UNAVAILABLE,
                "AUTHENTICATION_UNAVAILABLE",
                "Authentication is temporarily unavailable.",
            ),
            Self::Database(_) | Self::Configuration(_) | Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTHENTICATION_UNAVAILABLE",
                "Authentication is temporarily unavailable.",
            ),
        };
        let mut body = json!({ "error": code, "message": message, "retryable": retryable });
        if let Some(next_action) = next_action {
            body["next_action"] = json!(next_action);
        }
        (
            status,
            [(CONTENT_TYPE, "application/json")],
            axum::Json(body),
        )
            .into_response()
    }
}

#[derive(Debug, Deserialize)]
struct EmailRequest {
    email: String,
    intent: Option<String>,
    invitation_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct EmailVerificationRequest {
    challenge_id: Uuid,
    email: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct InvitationRequest {
    email: String,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TenantProfileRequest {
    display_name: String,
}

#[derive(Debug, Serialize)]
struct TenantSetupStep {
    id: &'static str,
    label: &'static str,
    status: &'static str,
    action_path: &'static str,
}

#[derive(Debug, Serialize)]
struct TenantReadiness {
    state: &'static str,
    completed_steps: i32,
    total_steps: i32,
    steps: Vec<TenantSetupStep>,
}

#[derive(Debug, Serialize)]
struct TenantContextResponse {
    schema: &'static str,
    tenant_id: Uuid,
    display_name: String,
    status: String,
    readiness: TenantReadiness,
}

#[derive(Debug, Serialize)]
struct TenantProfileMutationResponse {
    schema: &'static str,
    mutation_id: Uuid,
    idempotent_replay: bool,
    tenant_id: Uuid,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct AdministratorPromotionResponse {
    schema: &'static str,
    mutation_id: Uuid,
    idempotent_replay: bool,
    user_id: Uuid,
    role: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ChallengeResponse {
    challenge_id: Uuid,
    expires_at_unix: i64,
    retry_after_seconds: i64,
    invitation_role: Option<String>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct UserDirectoryEntry {
    id: Uuid,
    email: String,
    status: String,
    created_at_unix: i64,
    last_verified_at_unix: i64,
    roles: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UserDirectoryResponse {
    schema: &'static str,
    users: Vec<UserDirectoryEntry>,
}

#[derive(Debug, Serialize)]
struct RoleMutationResponse {
    schema: &'static str,
    mutation_id: Uuid,
    idempotent_replay: bool,
    user_id: Uuid,
    role: String,
    action: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct InvitationResponse {
    schema: &'static str,
    invitation_id: Uuid,
    idempotent_replay: bool,
    email: String,
    role: String,
    status: String,
    expires_at_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RoleCatalogDocument {
    schema: String,
    version: u16,
    roles: Vec<RoleCatalogEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RoleCatalogEntry {
    id: String,
    label: String,
    description: String,
    assignable: bool,
    scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RoleCatalogResponse {
    schema: &'static str,
    roles: Vec<RoleCatalogPublicEntry>,
}

#[derive(Debug, Serialize)]
struct RoleCatalogPublicEntry {
    id: String,
    label: String,
    description: String,
    assignable: bool,
}

const ROLE_CATALOG_SCHEMA: &str = "authorization-roles/v1";
const ROLE_CATALOG_RESPONSE_SCHEMA: &str = "auth-role-catalog/v1";
const ROLE_CATALOG_MAX_ROLES: usize = 32;
const ROLE_CATALOG_MAX_VALUE_LENGTH: usize = 128;
type ParsedRoleCatalog = (Arc<[RoleCatalogEntry]>, Arc<[RoleScopeProfile]>);

fn valid_role_catalog_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= ROLE_CATALOG_MAX_VALUE_LENGTH
        && value
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'))
}

fn valid_role_catalog_scope(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= ROLE_CATALOG_MAX_VALUE_LENGTH
        && !value.starts_with(':')
        && !value.ends_with(':')
        && !value.contains("::")
        && value
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b':' | b'_' | b'-'))
}

fn parse_role_catalog() -> Result<ParsedRoleCatalog, ServiceError> {
    let document: RoleCatalogDocument = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/authorization-roles-v1.json"
    )))
    .map_err(|_| {
        ServiceError::Configuration("authorization role catalog is invalid".to_string())
    })?;
    if document.schema != ROLE_CATALOG_SCHEMA
        || document.version != 1
        || document.roles.is_empty()
        || document.roles.len() > ROLE_CATALOG_MAX_ROLES
    {
        return Err(ServiceError::Configuration(
            "authorization role catalog is invalid".to_string(),
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for role in &document.roles {
        if !valid_role_catalog_value(&role.id)
            || !ids.insert(role.id.clone())
            || role.label.is_empty()
            || role.label.len() > ROLE_CATALOG_MAX_VALUE_LENGTH
            || role.description.is_empty()
            || role.description.len() > 512
            || role.scopes.len() > 16
            || role
                .scopes
                .iter()
                .any(|scope| !valid_role_catalog_scope(scope))
        {
            return Err(ServiceError::Configuration(
                "authorization role catalog is invalid".to_string(),
            ));
        }
        if role.id == "administrator" && role.assignable {
            return Err(ServiceError::Configuration(
                "administrator cannot be assignable in the authorization role catalog".to_string(),
            ));
        }
    }
    let profiles = document
        .roles
        .iter()
        .map(|role| RoleScopeProfile {
            role: role.id.clone(),
            scopes: role.scopes.clone(),
        })
        .collect::<Vec<_>>();
    Ok((document.roles.into(), profiles.into()))
}

#[derive(Debug, Serialize)]
struct InvitationDirectoryEntry {
    id: Uuid,
    email: String,
    role: String,
    status: String,
    created_at_unix: i64,
    expires_at_unix: i64,
    accepted_at_unix: Option<i64>,
    revoked_at_unix: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InvitationDirectoryResponse {
    schema: &'static str,
    invitations: Vec<InvitationDirectoryEntry>,
}

#[derive(Clone, Debug)]
struct EmailMessage {
    challenge_id: Uuid,
    recipient: String,
    code: String,
    expires_at_unix: i64,
}

#[derive(Clone, Debug)]
struct InvitationEmail {
    invitation_id: Uuid,
    recipient: String,
    role: String,
    expires_at_unix: i64,
}

#[derive(Debug, Error)]
enum EmailError {
    #[error("local inbox failed")]
    LocalInbox(#[source] std::io::Error),
    #[error("provider transport failed")]
    ProviderTransport,
    #[error("provider rejected the email")]
    ProviderRejected,
    #[error("provider response was invalid")]
    ProviderResponse,
    #[error("SMTP transport failed")]
    SmtpTransport,
    #[error("SMTP message was invalid")]
    SmtpMessage,
}

#[async_trait]
trait EmailSender: Send + Sync {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError>;
    async fn send_invitation(&self, message: InvitationEmail) -> Result<(), EmailError>;
}

#[derive(Clone)]
struct LocalInboxSender {
    directory: Arc<PathBuf>,
}

#[derive(Serialize)]
struct LocalInboxMessage<'a> {
    schema: &'static str,
    challenge_id: Uuid,
    recipient: &'a str,
    code: &'a str,
    expires_at_unix: i64,
}

#[derive(Serialize)]
struct LocalInvitationMessage<'a> {
    schema: &'static str,
    invitation_id: Uuid,
    recipient: &'a str,
    role: &'a str,
    expires_at_unix: i64,
}

#[async_trait]
impl EmailSender for LocalInboxSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        tokio::fs::create_dir_all(self.directory.as_ref())
            .await
            .map_err(EmailError::LocalInbox)?;
        let payload = serde_json::to_vec_pretty(&LocalInboxMessage {
            schema: "solana-tax-protocol/local-email-inbox/v1",
            challenge_id: message.challenge_id,
            recipient: &message.recipient,
            code: &message.code,
            expires_at_unix: message.expires_at_unix,
        })
        .map_err(|_| {
            EmailError::LocalInbox(std::io::Error::other("local email serialization failed"))
        })?;
        let path = self
            .directory
            .join(format!("{}.json", message.challenge_id));
        tokio::fs::write(path, payload)
            .await
            .map_err(EmailError::LocalInbox)
    }

    async fn send_invitation(&self, message: InvitationEmail) -> Result<(), EmailError> {
        tokio::fs::create_dir_all(self.directory.as_ref())
            .await
            .map_err(EmailError::LocalInbox)?;
        let payload = serde_json::to_vec_pretty(&LocalInvitationMessage {
            schema: "solana-tax-protocol/local-invitation-inbox/v1",
            invitation_id: message.invitation_id,
            recipient: &message.recipient,
            role: &message.role,
            expires_at_unix: message.expires_at_unix,
        })
        .map_err(|_| {
            EmailError::LocalInbox(std::io::Error::other(
                "local invitation serialization failed",
            ))
        })?;
        let path = self
            .directory
            .join(format!("invitation-{}.json", message.invitation_id));
        tokio::fs::write(path, payload)
            .await
            .map_err(EmailError::LocalInbox)
    }
}

#[derive(Clone)]
struct ProviderEmailSender {
    client: reqwest::Client,
    endpoint: Url,
    api_key: Arc<str>,
    from: Arc<str>,
    app_base_url: Arc<str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SmtpSecurity {
    StartTls,
    Tls,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SmtpAuthentication {
    AppPassword,
    IpAllowlist,
}

struct GoogleSmtpConfig {
    host: String,
    port: u16,
    security: SmtpSecurity,
    authentication: SmtpAuthentication,
    username: Option<Arc<str>>,
    password: Option<Arc<str>>,
}

struct GoogleSmtpEmailSender {
    transport: Arc<AsyncSmtpTransport<Tokio1Executor>>,
    from: Mailbox,
    app_base_url: Arc<str>,
}

#[derive(Debug, Serialize)]
struct ProviderEmailPayload {
    from: String,
    to: Vec<String>,
    subject: String,
    text: String,
    html: String,
}

impl ProviderEmailPayload {
    fn new(
        from: impl Into<String>,
        recipient: impl Into<String>,
        subject: impl Into<String>,
        text: String,
        html: String,
    ) -> Self {
        Self {
            from: from.into(),
            to: vec![recipient.into()],
            subject: subject.into(),
            text,
            html,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ProviderEmailResponse {
    id: String,
}

struct RenderedEmail {
    text: String,
    html: String,
}

impl ProviderEmailSender {
    async fn dispatch(
        &self,
        payload: ProviderEmailPayload,
        idempotency_key: &str,
    ) -> Result<(), EmailError> {
        let body = serde_json::to_vec(&payload).map_err(|_| EmailError::ProviderResponse)?;
        if body.len() > MAX_BODY_BYTES {
            return Err(EmailError::ProviderResponse);
        }
        let response = self
            .client
            .post(self.endpoint.clone())
            .bearer_auth(self.api_key.as_ref())
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .header("Idempotency-Key", idempotency_key)
            .body(body)
            .send()
            .await
            .map_err(|_| EmailError::ProviderTransport)?;
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
                .is_some_and(|length| length > MAX_PROVIDER_RESPONSE_BYTES as u64)
        {
            return Err(EmailError::ProviderRejected);
        }
        let body = read_provider_response(response).await?;
        let response: ProviderEmailResponse =
            serde_json::from_slice(&body).map_err(|_| EmailError::ProviderResponse)?;
        if response.id.trim().is_empty()
            || response.id.len() > 256
            || response.id.chars().any(char::is_control)
        {
            return Err(EmailError::ProviderResponse);
        }
        Ok(())
    }
}

#[async_trait]
impl EmailSender for ProviderEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        let rendered = render_otp_email(&message.code);
        let payload = ProviderEmailPayload::new(
            self.from.as_ref(),
            message.recipient,
            "Your Solana Tax Protocol verification code",
            rendered.text,
            rendered.html,
        );
        self.dispatch(payload, &format!("auth-otp-{}", message.challenge_id))
            .await
    }

    async fn send_invitation(&self, message: InvitationEmail) -> Result<(), EmailError> {
        let rendered = render_invitation_email(
            self.app_base_url.as_ref(),
            message.invitation_id,
            &message.role,
        );
        let payload = ProviderEmailPayload::new(
            self.from.as_ref(),
            message.recipient,
            "You have been invited to Solana Tax Protocol",
            rendered.text,
            rendered.html,
        );
        self.dispatch(
            payload,
            &format!("auth-invitation-{}", message.invitation_id),
        )
        .await
    }
}

#[async_trait]
impl EmailSender for GoogleSmtpEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), EmailError> {
        let rendered = render_otp_email(&message.code);
        let email = build_smtp_message(
            &self.from,
            &message.recipient,
            "Your Solana Tax Protocol verification code",
            rendered,
        )?;
        send_google_smtp_message(&self.transport, email).await
    }

    async fn send_invitation(&self, message: InvitationEmail) -> Result<(), EmailError> {
        let rendered = render_invitation_email(
            self.app_base_url.as_ref(),
            message.invitation_id,
            &message.role,
        );
        let email = build_smtp_message(
            &self.from,
            &message.recipient,
            "You have been invited to Solana Tax Protocol",
            rendered,
        )?;
        send_google_smtp_message(&self.transport, email).await
    }
}

async fn send_google_smtp_message(
    transport: &AsyncSmtpTransport<Tokio1Executor>,
    email: Message,
) -> Result<(), EmailError> {
    match transport.send(email).await {
        Ok(_) => Ok(()),
        Err(error) => {
            warn!(
                smtp_error_class = classify_smtp_error(&error),
                "Google SMTP delivery failed"
            );
            Err(EmailError::SmtpTransport)
        }
    }
}

fn classify_smtp_error(error: &lettre::transport::smtp::Error) -> &'static str {
    if error.is_timeout() {
        "timeout"
    } else if error.is_tls() {
        "tls"
    } else if error.is_permanent() {
        "permanent_response"
    } else if error.is_transient() {
        "transient_response"
    } else if error.is_response() {
        "response"
    } else if error.is_client() {
        "client"
    } else if error.is_transport_shutdown() {
        "transport_shutdown"
    } else {
        "connection_or_network"
    }
}

fn build_smtp_message(
    from: &Mailbox,
    recipient: &str,
    subject: &str,
    rendered: RenderedEmail,
) -> Result<Message, EmailError> {
    let to = recipient
        .parse::<Mailbox>()
        .map_err(|_| EmailError::SmtpMessage)?;
    Message::builder()
        .from(from.clone())
        .to(to)
        .subject(subject)
        .multipart(MultiPart::alternative_plain_html(
            rendered.text,
            rendered.html,
        ))
        .map_err(|_| EmailError::SmtpMessage)
}

async fn read_provider_response(mut response: reqwest::Response) -> Result<Vec<u8>, EmailError> {
    let mut body = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or_default()
            .min(MAX_PROVIDER_RESPONSE_BYTES as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| EmailError::ProviderTransport)?
    {
        if body.len().saturating_add(chunk.len()) > MAX_PROVIDER_RESPONSE_BYTES {
            return Err(EmailError::ProviderResponse);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn render_otp_email(code: &str) -> RenderedEmail {
    let escaped_code = escape_html(code);
    RenderedEmail {
        text: format!(
            "Your Solana Tax Protocol verification code is {code}.\n\nThis code expires shortly. If you did not request it, you can ignore this email."
        ),
        html: format!(
            "<p>Your Solana Tax Protocol verification code is <strong>{escaped_code}</strong>.</p><p>This code expires shortly. If you did not request it, you can ignore this email.</p>"
        ),
    }
}

fn render_invitation_email(app_base_url: &str, invitation_id: Uuid, role: &str) -> RenderedEmail {
    let escaped_role = escape_html(role);
    let signup_url = format!(
        "{}/auth/signup?invite={invitation_id}",
        app_base_url.trim_end_matches('/')
    );
    let escaped_url = escape_html(&signup_url);
    RenderedEmail {
        text: format!(
            "You have been invited to join Solana Tax Protocol as {role}.\n\nOpen {signup_url} and use this email address to continue.\nInvitation reference: {invitation_id}."
        ),
        html: format!(
            "<p>You have been invited to join Solana Tax Protocol as <strong>{escaped_role}</strong>.</p><p><a href=\"{escaped_url}\">Open Solana Tax Protocol</a> and use this email address to continue.</p><p>Invitation reference: {invitation_id}.</p>"
        ),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn now_unix() -> Result<i64, ServiceError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .map_err(|_| ServiceError::Unavailable)
}

fn source_digest(pepper: &[u8], source: &str) -> Vec<u8> {
    otp_digest(pepper, Uuid::nil(), source, "source").to_vec()
}

fn parse_bool(raw: &str, key: &str) -> Result<bool, ServiceError> {
    match raw {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ServiceError::Configuration(format!(
            "{key} must be true or false"
        ))),
    }
}

fn configured_local_nonnegative_seconds(
    key: &str,
    default: i64,
    environment: &str,
) -> Result<i64, ServiceError> {
    configured_local_nonnegative_seconds_from(
        key,
        env::var(key).ok().as_deref(),
        default,
        environment,
    )
}

fn configured_local_nonnegative_seconds_from(
    key: &str,
    raw: Option<&str>,
    default: i64,
    environment: &str,
) -> Result<i64, ServiceError> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    if environment != "local" {
        return Err(ServiceError::Configuration(format!("{key} is local-only")));
    }
    let value = raw.parse::<i64>().map_err(|_| {
        ServiceError::Configuration(format!("{key} must be a non-negative integer"))
    })?;
    if value < 0 {
        return Err(ServiceError::Configuration(format!(
            "{key} must be a non-negative integer"
        )));
    }
    Ok(value)
}

fn configured_otp_rate_limit_window_seconds(
    environment: &str,
    default: i64,
) -> Result<i64, ServiceError> {
    let raw = env::var("AUTH_OTP_RATE_LIMIT_WINDOW_SECONDS").ok();
    configured_otp_rate_limit_window_seconds_from(raw.as_deref(), default, environment)
}

fn configured_otp_rate_limit_window_seconds_from(
    raw: Option<&str>,
    default: i64,
    environment: &str,
) -> Result<i64, ServiceError> {
    let value = raw
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                ServiceError::Configuration(
                    "AUTH_OTP_RATE_LIMIT_WINDOW_SECONDS must be a bounded integer".to_string(),
                )
            })
        })
        .transpose()?
        .unwrap_or(default);
    let minimum = if environment == "local" { 60 } else { 300 };
    if !(minimum..=3600).contains(&value) {
        return Err(ServiceError::Configuration(format!(
            "AUTH_OTP_RATE_LIMIT_WINDOW_SECONDS must be between {minimum} and 3600 seconds"
        )));
    }
    Ok(value)
}

fn configured_self_signup_daily_limit_from(
    raw: Option<&str>,
    default: i64,
    environment: &str,
) -> Result<i64, ServiceError> {
    let value = raw
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                ServiceError::Configuration(
                    "AUTH_SELF_SIGNUP_DAILY_LIMIT must be a bounded integer".to_string(),
                )
            })
        })
        .transpose()?
        .unwrap_or(default);
    let minimum = if environment == "local" { 0 } else { 1 };
    if !(minimum..=10_000).contains(&value) {
        return Err(ServiceError::Configuration(format!(
            "AUTH_SELF_SIGNUP_DAILY_LIMIT must be between {minimum} and 10000"
        )));
    }
    Ok(value)
}

fn parse_seed(raw: &[u8]) -> Result<[u8; 32], ServiceError> {
    if raw.len() == 32 {
        return raw
            .try_into()
            .map_err(|_| ServiceError::Configuration("signing key length is invalid".to_string()));
    }
    let text = std::str::from_utf8(raw)
        .map_err(|_| {
            ServiceError::Configuration("signing key is not UTF-8 or raw 32 bytes".to_string())
        })?
        .trim();
    if text.len() != 64 {
        return Err(ServiceError::Configuration(
            "signing key must be 32 raw bytes or 64 hex characters".to_string(),
        ));
    }
    let mut seed = [0_u8; 32];
    let (pairs, remainder) = text.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(ServiceError::Configuration(
            "signing key contains invalid hex".to_string(),
        ));
    }
    for (index, pair) in pairs.iter().enumerate() {
        let high = (pair[0] as char).to_digit(16).ok_or_else(|| {
            ServiceError::Configuration("signing key contains invalid hex".to_string())
        })?;
        let low = (pair[1] as char).to_digit(16).ok_or_else(|| {
            ServiceError::Configuration("signing key contains invalid hex".to_string())
        })?;
        seed[index] = ((high << 4) | low) as u8;
    }
    Ok(seed)
}

fn configured_bytes(
    key: &str,
    local_default: Option<&[u8]>,
    environment: &str,
) -> Result<Vec<u8>, ServiceError> {
    let file_key = format!("{key}_FILE");
    if let Some(path) = env::var_os(&file_key) {
        let path = PathBuf::from(path);
        let bytes = fs::read(&path).map_err(|_| {
            ServiceError::Configuration(format!("{file_key} file could not be read"))
        })?;
        if bytes.is_empty() {
            return Err(ServiceError::Configuration(format!(
                "{file_key} file is empty"
            )));
        }
        return Ok(bytes);
    }
    let value = env::var(key).unwrap_or_default();
    if !value.is_empty() {
        return Ok(value.into_bytes());
    }
    if environment == "local"
        && let Some(default) = local_default
    {
        return Ok(default.to_vec());
    }
    Err(ServiceError::Configuration(format!("{key} is required")))
}

fn parse_list(raw: &str, key: &str) -> Result<Arc<[String]>, ServiceError> {
    let values = raw
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() || values.len() > 32 || values.iter().any(|value| value.len() > 128) {
        return Err(ServiceError::Configuration(format!(
            "{key} must contain 1 to 32 bounded values"
        )));
    }
    Ok(values.into())
}

fn configured_role_scopes(
    key: &str,
    environment: &str,
    local_default: &str,
) -> Result<Arc<[String]>, ServiceError> {
    match env::var(key) {
        Ok(value) => parse_list(&value, key),
        Err(_) if environment == "local" => parse_list(local_default, key),
        Err(_) => Err(ServiceError::Configuration(format!(
            "{key} is required outside local"
        ))),
    }
}

fn build_pool(database_url: &str, pool_size: usize) -> Result<Pool, ServiceError> {
    let mut config = PoolConfig::new();
    config.url = Some(database_url.to_string());
    config.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    config.pool = Some(deadpool_postgres::PoolConfig::new(pool_size));
    config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .map_err(|error| {
            ServiceError::Configuration(format!("database pool configuration failed: {error}"))
        })
}

async fn healthz() -> impl IntoResponse {
    axum::Json(HealthResponse {
        service: "auth-service",
        status: "ok",
    })
}

async fn readyz(State(state): State<AppState>) -> Result<impl IntoResponse, ServiceError> {
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    client
        .query_one("SELECT 1", &[])
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    Ok(axum::Json(HealthResponse {
        service: "auth-service",
        status: "ready",
    }))
}

async fn jwks(State(state): State<AppState>) -> Result<impl IntoResponse, ServiceError> {
    let body = state
        .issuer
        .jwks_json()
        .map_err(|_| ServiceError::Internal)?;
    Ok(([(CONTENT_TYPE, "application/json")], body))
}

const MANAGED_ROLE: &str = "moderator";

fn is_assignable_role(state: &AppState, role: &str) -> bool {
    state
        .role_catalog
        .iter()
        .any(|entry| entry.id == role && entry.assignable)
}

async fn role_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    let _principal = require_admin(&state, &headers).await?;
    let roles = state
        .role_catalog
        .iter()
        .map(|role| RoleCatalogPublicEntry {
            id: role.id.clone(),
            label: role.label.clone(),
            description: role.description.clone(),
            assignable: role.assignable,
        })
        .collect();
    Ok(axum::Json(RoleCatalogResponse {
        schema: ROLE_CATALOG_RESPONSE_SCHEMA,
        roles,
    }))
}

fn authenticated_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<VerifiedAccessToken, ServiceError> {
    let value = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(ServiceError::AuthorizationFailed)?;
    let token = value
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty() && !value.contains(char::is_whitespace))
        .ok_or(ServiceError::AuthorizationFailed)?;
    state
        .issuer
        .verify_access_token(token, now_unix()?)
        .map_err(|_| ServiceError::AuthorizationFailed)
}

fn validate_tenant_display_name(value: &str) -> Result<String, ServiceError> {
    let name = value.trim();
    if name.is_empty()
        || name.len() > 120
        || name.chars().any(char::is_control)
        || name.contains('\n')
        || name.contains('\r')
    {
        return Err(ServiceError::BadRequest);
    }
    Ok(name.to_string())
}

fn tenant_step_status(complete: bool, active: bool) -> &'static str {
    if !active {
        "blocked"
    } else if complete {
        "complete"
    } else {
        "pending"
    }
}

async fn tenant_context(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = authenticated_principal(&state, &headers)?;
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let row = client
        .query_opt(
            "SELECT t.display_name, t.status,\
                    (t.display_name <> 'Untitled workspace'),\
                    EXISTS (SELECT 1 FROM settlement_routes sr\
                            WHERE sr.merchant_id = t.id\
                              AND sr.status = 'active'\
                              AND sr.verification_digest IS NOT NULL\
                              AND sr.effective_from <= now()\
                              AND (sr.effective_until IS NULL OR sr.effective_until > now())),\
                    EXISTS (SELECT 1\
                            FROM jurisdiction_policies jp\
                            JOIN rule_sets rs ON rs.id = jp.rule_set_id AND rs.merchant_id = jp.merchant_id\
                            WHERE jp.merchant_id = t.id\
                              AND jp.status = 'published'\
                              AND rs.status = 'published'\
                              AND jp.effective_from <= now()\
                              AND (jp.effective_until IS NULL OR jp.effective_until > now())\
                              AND rs.effective_from <= now()\
                              AND (rs.effective_until IS NULL OR rs.effective_until > now()))\
             FROM auth_tenants t WHERE t.id = $1",
            &[&principal.merchant_id],
        )
        .await
        .map_err(|error| {
            error!(error = ?error, "tenant context query failed");
            ServiceError::Database(error)
        })?
        .ok_or(ServiceError::NotFound)?;
    let display_name: String = row.get(0);
    let status: String = row.get(1);
    let active = status == "active";
    let profile_complete: bool = row.get(2);
    let route_complete: bool = row.get(3);
    let policy_complete: bool = row.get(4);
    let complete = [profile_complete, route_complete, policy_complete]
        .into_iter()
        .filter(|value| *value)
        .count() as i32;
    let steps = vec![
        TenantSetupStep {
            id: "workspace_profile",
            label: "Name your workspace",
            status: tenant_step_status(profile_complete, active),
            action_path: "/admin/settings",
        },
        TenantSetupStep {
            id: "settlement_route",
            label: "Verify a settlement route",
            status: tenant_step_status(route_complete, active),
            action_path: "/admin/settlements",
        },
        TenantSetupStep {
            id: "tax_policy",
            label: "Publish a tax policy",
            status: tenant_step_status(policy_complete, active),
            action_path: "/admin/tax/rules",
        },
    ];
    let state_name = if !active {
        "blocked"
    } else if complete == 0 {
        "not_started"
    } else if complete == 3 {
        "ready"
    } else {
        "in_progress"
    };
    Ok(axum::Json(TenantContextResponse {
        schema: "auth-tenant-context/v1",
        tenant_id: principal.merchant_id,
        display_name,
        status,
        readiness: TenantReadiness {
            state: state_name,
            completed_steps: complete,
            total_steps: 3,
            steps,
        },
    }))
}

async fn update_tenant_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<TenantProfileRequest>,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    let display_name = validate_tenant_display_name(&request.display_name)?;
    let idempotency_key = parse_idempotency_key(&headers)?;
    let mutation_id = Uuid::now_v7();
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    if let Some(existing) = transaction
        .query_opt(
            "SELECT id, merchant_id, display_name FROM auth_tenant_profile_mutations WHERE idempotency_key = $1 FOR UPDATE",
            &[&idempotency_key],
        )
        .await
        .map_err(ServiceError::Database)?
    {
        let existing_tenant: Uuid = existing.get(1);
        let existing_name: String = existing.get(2);
        if existing_tenant != principal.merchant_id || existing_name != display_name {
            return Err(ServiceError::Conflict);
        }
        let existing_id: Uuid = existing.get(0);
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Ok(axum::Json(TenantProfileMutationResponse {
            schema: "auth-tenant-profile-mutation/v1",
            mutation_id: existing_id,
            idempotent_replay: true,
            tenant_id: principal.merchant_id,
            display_name,
        }));
    }
    let actor = transaction
        .query_opt(
            "SELECT status FROM auth_users WHERE id = $1 AND merchant_id = $2 FOR UPDATE",
            &[&principal.subject, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    if actor.is_none_or(|row| row.get::<_, String>(0) != "active") {
        return Err(ServiceError::Forbidden);
    }
    let updated = transaction
        .execute(
            "UPDATE auth_tenants SET display_name = $1, updated_at = now() WHERE id = $2 AND status = 'active'",
            &[&display_name, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    if updated != 1 {
        return Err(ServiceError::Forbidden);
    }
    transaction
        .execute(
            "INSERT INTO auth_tenant_profile_mutations (id, idempotency_key, actor_id, merchant_id, display_name) VALUES ($1, $2, $3, $4, $5)",
            &[&mutation_id, &idempotency_key, &principal.subject, &principal.merchant_id, &display_name],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('tenant_profile_updated', $1, 'accepted', $2)",
            &[&principal.subject, &json!({ "merchant_id": principal.merchant_id, "mutation_id": mutation_id })],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction.commit().await.map_err(ServiceError::Database)?;
    Ok(axum::Json(TenantProfileMutationResponse {
        schema: "auth-tenant-profile-mutation/v1",
        mutation_id,
        idempotent_replay: false,
        tenant_id: principal.merchant_id,
        display_name,
    }))
}

async fn promote_administrator(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    let idempotency_key = parse_idempotency_key(&headers)?;
    let mutation_id = Uuid::now_v7();
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    if let Some(existing) = transaction
        .query_opt(
            "SELECT id, target_id FROM auth_administrator_mutations WHERE idempotency_key = $1 FOR UPDATE",
            &[&idempotency_key],
        )
        .await
        .map_err(ServiceError::Database)?
    {
        if existing.get::<_, Uuid>(1) != user_id {
            return Err(ServiceError::Conflict);
        }
        let existing_id: Uuid = existing.get(0);
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Ok(axum::Json(AdministratorPromotionResponse {
            schema: "auth-administrator-promotion/v1",
            mutation_id: existing_id,
            idempotent_replay: true,
            user_id,
            role: "administrator",
            status: "accepted",
        }));
    }
    let target = transaction
        .query_opt(
            "SELECT status, merchant_id FROM auth_users WHERE id = $1 FOR UPDATE",
            &[&user_id],
        )
        .await
        .map_err(ServiceError::Database)?
        .ok_or(ServiceError::NotFound)?;
    if target.get::<_, String>(0) != "active" || target.get::<_, Uuid>(1) != principal.merchant_id {
        return Err(ServiceError::NotFound);
    }
    transaction
        .execute(
            "INSERT INTO auth_user_roles (user_id, role, status, assigned_by) VALUES ($1, 'administrator', 'active', $2) ON CONFLICT (user_id, role) DO UPDATE SET status = 'active', assigned_at = now(), assigned_by = EXCLUDED.assigned_by",
            &[&user_id, &principal.subject],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_administrator_mutations (id, idempotency_key, actor_id, target_id, merchant_id, action) VALUES ($1, $2, $3, $4, $5, 'promote')",
            &[&mutation_id, &idempotency_key, &principal.subject, &user_id, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('administrator_promoted', $1, 'accepted', $2)",
            &[&user_id, &json!({ "actor_id": principal.subject, "merchant_id": principal.merchant_id, "mutation_id": mutation_id })],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction.commit().await.map_err(ServiceError::Database)?;
    Ok(axum::Json(AdministratorPromotionResponse {
        schema: "auth-administrator-promotion/v1",
        mutation_id,
        idempotent_replay: false,
        user_id,
        role: "administrator",
        status: "accepted",
    }))
}

async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<VerifiedAccessToken, ServiceError> {
    let principal = authenticated_principal(state, headers)?;
    if !principal
        .scopes
        .iter()
        .any(|scope| scope == "identity:role:manage")
    {
        return Err(ServiceError::Forbidden);
    }
    if !principal.roles.iter().any(|role| role == "administrator") {
        return Err(ServiceError::Forbidden);
    }
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let durable_admin = client
        .query_opt(
            "SELECT u.status FROM auth_users u WHERE u.id = $1 AND u.merchant_id = $2 \
             AND EXISTS (SELECT 1 FROM auth_tenants t WHERE t.id = u.merchant_id AND t.status = 'active') \
             AND EXISTS (SELECT 1 FROM auth_user_roles r WHERE r.user_id = u.id AND r.role = 'administrator' AND r.status = 'active')",
            &[&principal.subject, &principal.merchant_id],
        )
        .await
        .map_err(|error| {
            error!(error = ?error, "durable administrator lookup failed");
            ServiceError::Database(error)
        })?;
    if durable_admin.is_none_or(|row| row.get::<_, String>(0) != "active") {
        return Err(ServiceError::Forbidden);
    }
    Ok(principal)
}

fn parse_idempotency_key(headers: &HeaderMap) -> Result<Uuid, ServiceError> {
    let raw = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .ok_or(ServiceError::BadRequest)?;
    let key = Uuid::parse_str(raw).map_err(|_| ServiceError::BadRequest)?;
    if key.get_version_num() != 7 {
        return Err(ServiceError::BadRequest);
    }
    Ok(key)
}

const INVITATION_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;

fn invitation_role(raw: Option<&str>) -> Result<String, ServiceError> {
    let role = raw
        .unwrap_or("merchant_operator")
        .trim()
        .to_ascii_lowercase();
    if role == "merchant_operator" || role == MANAGED_ROLE {
        Ok(role)
    } else {
        Err(ServiceError::BadRequest)
    }
}

fn system_time_unix(value: SystemTime) -> Result<i64, ServiceError> {
    value
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .map_err(|_| ServiceError::Unavailable)
}

async fn create_invitation(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<InvitationRequest>,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    let email = normalize_email(&request.email).map_err(|_| ServiceError::BadRequest)?;
    let role = invitation_role(request.role.as_deref())?;
    let idempotency_key = parse_idempotency_key(&headers)?;
    let invitation_id = Uuid::now_v7();
    let expires_at_unix = now_unix()?.saturating_add(INVITATION_TTL_SECONDS);
    let expires_at = SystemTime::now()
        .checked_add(Duration::from_secs(INVITATION_TTL_SECONDS as u64))
        .ok_or(ServiceError::Unavailable)?;
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    let actor = transaction
        .query_opt(
            "SELECT u.status FROM auth_users u WHERE u.id = $1 AND u.merchant_id = $2 \
             AND EXISTS (SELECT 1 FROM auth_user_roles r WHERE r.user_id = u.id AND r.role = 'administrator' AND r.status = 'active') FOR UPDATE",
            &[&principal.subject, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    if actor.is_none_or(|row| row.get::<_, String>(0) != "active") {
        return Err(ServiceError::Forbidden);
    }
    if let Some(existing) = transaction
        .query_opt(
            "SELECT id, email_normalized, role, status, expires_at FROM auth_tenant_invitations WHERE idempotency_key = $1 FOR UPDATE",
            &[&idempotency_key],
        )
        .await
        .map_err(ServiceError::Database)?
    {
        let existing_email: String = existing.get(1);
        let existing_role: String = existing.get(2);
        let existing_id: Uuid = existing.get(0);
        let existing_status: String = existing.get(3);
        let existing_expires_at: SystemTime = existing.get(4);
        if existing_email != email || existing_role != role {
            return Err(ServiceError::InvitationConflict);
        }
        let replay_expires_at_unix = system_time_unix(existing_expires_at)?;
        transaction.commit().await.map_err(ServiceError::Database)?;
        if existing_status == "pending" {
            state
                .email_sender
                .send_invitation(InvitationEmail {
                    invitation_id: existing_id,
                    recipient: email.clone(),
                    role: role.clone(),
                    expires_at_unix: replay_expires_at_unix,
                })
                .await
                .map_err(|_| ServiceError::EmailDelivery)?;
        }
        return Ok((
            StatusCode::ACCEPTED,
            axum::Json(InvitationResponse {
                schema: "auth-invitation/v1",
                invitation_id: existing_id,
                idempotent_replay: true,
                email,
                role,
                status: existing_status,
                expires_at_unix: replay_expires_at_unix,
            }),
        ));
    }
    if transaction
        .query_opt(
            "SELECT id FROM auth_users WHERE email_normalized = $1",
            &[&email],
        )
        .await
        .map_err(ServiceError::Database)?
        .is_some()
    {
        return Err(ServiceError::InvitationConflict);
    }
    if transaction
        .query_opt(
            "SELECT id FROM auth_tenant_invitations WHERE merchant_id = $1 AND email_normalized = $2 AND status = 'pending' FOR UPDATE",
            &[&principal.merchant_id, &email],
        )
        .await
        .map_err(ServiceError::Database)?
        .is_some()
    {
        return Err(ServiceError::InvitationConflict);
    }
    transaction
        .execute(
            "INSERT INTO auth_tenant_invitations (id, idempotency_key, merchant_id, email_normalized, role, invited_by, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&invitation_id, &idempotency_key, &principal.merchant_id, &email, &role, &principal.subject, &expires_at],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('invitation_created', $1, 'accepted', $2)",
            &[&principal.subject, &json!({ "invitation_id": invitation_id, "merchant_id": principal.merchant_id, "email_domain": email.rsplit('@').next().unwrap_or("unknown"), "role": role })],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction.commit().await.map_err(ServiceError::Database)?;
    state
        .email_sender
        .send_invitation(InvitationEmail {
            invitation_id,
            recipient: email.clone(),
            role: role.clone(),
            expires_at_unix,
        })
        .await
        .map_err(|_| ServiceError::EmailDelivery)?;
    Ok((
        StatusCode::ACCEPTED,
        axum::Json(InvitationResponse {
            schema: "auth-invitation/v1",
            invitation_id,
            idempotent_replay: false,
            email,
            role,
            status: "pending".to_string(),
            expires_at_unix,
        }),
    ))
}

async fn list_invitations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let rows = client
        .query(
            r#"SELECT id, email_normalized, role,
                    CASE WHEN status = 'pending' AND expires_at <= now() THEN 'expired' ELSE status END,
                    (extract(epoch FROM created_at))::bigint,
                    (extract(epoch FROM expires_at))::bigint,
                    (extract(epoch FROM accepted_at))::bigint,
                    (extract(epoch FROM revoked_at))::bigint
             FROM auth_tenant_invitations
             WHERE merchant_id = $1
             ORDER BY created_at DESC, id DESC
             LIMIT 100"#,
            &[&principal.merchant_id],
        )
        .await
        .map_err(|error| {
            error!(error = ?error, "auth invitation directory query failed");
            ServiceError::Database(error)
        })?;
    let invitations = rows
        .into_iter()
        .map(|row| InvitationDirectoryEntry {
            id: row.get(0),
            email: row.get(1),
            role: row.get(2),
            status: row.get(3),
            created_at_unix: row.get(4),
            expires_at_unix: row.get(5),
            accepted_at_unix: row.get(6),
            revoked_at_unix: row.get(7),
        })
        .collect();
    Ok(axum::Json(InvitationDirectoryResponse {
        schema: "auth-invitation-directory/v1",
        invitations,
    }))
}

async fn revoke_invitation(
    State(state): State<AppState>,
    Path(invitation_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    let idempotency_key = parse_idempotency_key(&headers)?;
    let mutation_id = Uuid::now_v7();
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    let actor = transaction
        .query_opt(
            "SELECT u.status FROM auth_users u WHERE u.id = $1 AND u.merchant_id = $2 \
             AND EXISTS (SELECT 1 FROM auth_user_roles r WHERE r.user_id = u.id AND r.role = 'administrator' AND r.status = 'active') FOR UPDATE",
            &[&principal.subject, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    if actor.is_none_or(|row| row.get::<_, String>(0) != "active") {
        return Err(ServiceError::Forbidden);
    }
    if let Some(existing) = transaction
        .query_opt(
            "SELECT id, invitation_id FROM auth_invitation_mutations WHERE idempotency_key = $1 FOR UPDATE",
            &[&idempotency_key],
        )
        .await
        .map_err(ServiceError::Database)?
    {
        let existing_invitation: Uuid = existing.get(1);
        if existing_invitation != invitation_id {
            return Err(ServiceError::InvitationConflict);
        }
        let invitation = transaction
            .query_one(
                "SELECT email_normalized, role, status, expires_at FROM auth_tenant_invitations WHERE id = $1 AND merchant_id = $2",
                &[&invitation_id, &principal.merchant_id],
            )
            .await
            .map_err(|error| {
                if error.code() == Some(&tokio_postgres::error::SqlState::NO_DATA_FOUND) {
                    ServiceError::NotFound
                } else {
                    ServiceError::Database(error)
                }
            })?;
        let expires_at_unix = system_time_unix(invitation.get(3))?;
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Ok((
            StatusCode::ACCEPTED,
            axum::Json(InvitationResponse {
                schema: "auth-invitation/v1",
                invitation_id,
                idempotent_replay: true,
                email: invitation.get(0),
                role: invitation.get(1),
                status: invitation.get(2),
                expires_at_unix,
            }),
        ));
    }
    let invitation = transaction
        .query_opt(
            "SELECT email_normalized, role, status, expires_at FROM auth_tenant_invitations WHERE id = $1 AND merchant_id = $2 FOR UPDATE",
            &[&invitation_id, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?
        .ok_or(ServiceError::NotFound)?;
    let email: String = invitation.get(0);
    let role: String = invitation.get(1);
    let status: String = invitation.get(2);
    let expires_at_unix = system_time_unix(invitation.get(3))?;
    if status != "pending" {
        return Err(ServiceError::InvitationConflict);
    }
    if expires_at_unix <= now_unix()? {
        transaction
            .execute(
                "UPDATE auth_tenant_invitations SET status = 'expired' WHERE id = $1",
                &[&invitation_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction
            .execute(
                "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('invitation_expired', $1, 'accepted', $2)",
                &[&principal.subject, &json!({ "invitation_id": invitation_id, "merchant_id": principal.merchant_id })],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Err(ServiceError::InvitationConflict);
    }
    transaction
        .execute(
            "UPDATE auth_tenant_invitations SET status = 'revoked', revoked_at = now() WHERE id = $1",
            &[&invitation_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_invitation_mutations (id, idempotency_key, actor_id, invitation_id, merchant_id, action) VALUES ($1, $2, $3, $4, $5, 'revoke')",
            &[&mutation_id, &idempotency_key, &principal.subject, &invitation_id, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('invitation_revoked', $1, 'accepted', $2)",
            &[&principal.subject, &json!({ "invitation_id": invitation_id, "merchant_id": principal.merchant_id, "email_domain": email.rsplit('@').next().unwrap_or("unknown") })],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction.commit().await.map_err(ServiceError::Database)?;
    Ok((
        StatusCode::ACCEPTED,
        axum::Json(InvitationResponse {
            schema: "auth-invitation/v1",
            invitation_id,
            idempotent_replay: false,
            email,
            role,
            status: "revoked".to_string(),
            expires_at_unix,
        }),
    ))
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let rows = client
        .query(
            "SELECT u.id, u.email_normalized, u.status,\
                    (extract(epoch FROM u.created_at))::bigint,\
                    (extract(epoch FROM u.last_verified_at))::bigint,\
                    COALESCE((SELECT array_agg(r.role ORDER BY r.role) \
                              FROM auth_user_roles r \
                              WHERE r.user_id = u.id AND r.status = 'active'), ARRAY[]::text[]) \
             FROM auth_users u \
             WHERE u.merchant_id = $1 \
             ORDER BY u.created_at DESC, u.id DESC \
             LIMIT 100",
            &[&principal.merchant_id],
        )
        .await
        .map_err(|error| {
            error!(error = ?error, "auth user directory query failed");
            ServiceError::Database(error)
        })?;
    let users = rows
        .into_iter()
        .map(|row| {
            let roles: Vec<String> = row.get(5);
            Ok(UserDirectoryEntry {
                id: row.get(0),
                email: row.get(1),
                status: row.get(2),
                created_at_unix: row.get(3),
                last_verified_at_unix: row.get(4),
                roles,
            })
        })
        .collect::<Result<Vec<_>, ServiceError>>()?;
    Ok(axum::Json(UserDirectoryResponse {
        schema: "auth-user-directory/v1",
        users,
    }))
}

async fn mutate_role(
    State(state): State<AppState>,
    Path((user_id, role)): Path<(Uuid, String)>,
    headers: HeaderMap,
    action: &'static str,
) -> Result<impl IntoResponse, ServiceError> {
    let principal = require_admin(&state, &headers).await?;
    if !is_assignable_role(&state, &role) {
        return Err(ServiceError::BadRequest);
    }
    let idempotency_key = parse_idempotency_key(&headers)?;
    let mutation_id = Uuid::now_v7();
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    let actor = transaction
        .query_opt(
            "SELECT u.status FROM auth_users u WHERE u.id = $1 AND u.merchant_id = $2 \
             AND EXISTS (SELECT 1 FROM auth_user_roles r WHERE r.user_id = u.id AND r.role = 'administrator' AND r.status = 'active') FOR UPDATE",
            &[&principal.subject, &principal.merchant_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    if actor.is_none_or(|row| row.get::<_, String>(0) != "active") {
        return Err(ServiceError::Forbidden);
    }
    if let Some(existing) = transaction
        .query_opt(
            "SELECT id, target_id, role, action FROM auth_role_mutations WHERE idempotency_key = $1 FOR UPDATE",
            &[&idempotency_key],
        )
        .await
        .map_err(ServiceError::Database)?
    {
        let existing_target: Uuid = existing.get(1);
        let existing_role: String = existing.get(2);
        let existing_action: String = existing.get(3);
        if existing_target != user_id || existing_role != role || existing_action != action {
            return Err(ServiceError::Conflict);
        }
        let existing_id: Uuid = existing.get(0);
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Ok(axum::Json(RoleMutationResponse {
            schema: "auth-role-mutation/v1",
            mutation_id: existing_id,
            idempotent_replay: true,
            user_id,
            role,
            action: action.to_string(),
            status: "accepted".to_string(),
        }));
    }
    let target = transaction
        .query_opt(
            "SELECT status, merchant_id FROM auth_users WHERE id = $1 FOR UPDATE",
            &[&user_id],
        )
        .await
        .map_err(ServiceError::Database)?
        .ok_or(ServiceError::NotFound)?;
    let target_status: String = target.get(0);
    let target_merchant: Uuid = target.get(1);
    if target_status != "active" || target_merchant != principal.merchant_id {
        return Err(ServiceError::NotFound);
    }
    let desired_status = if action == "assign" {
        "active"
    } else {
        "revoked"
    };
    transaction
        .execute(
            "INSERT INTO auth_user_roles (user_id, role, status, assigned_by) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id, role) DO UPDATE SET status = EXCLUDED.status, assigned_at = now(), assigned_by = EXCLUDED.assigned_by",
            &[&user_id, &role, &desired_status, &principal.subject],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_role_mutations (id, idempotency_key, actor_id, target_id, merchant_id, role, action) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&mutation_id, &idempotency_key, &principal.subject, &user_id, &principal.merchant_id, &role, &action],
        )
        .await
        .map_err(ServiceError::Database)?;
    let event_type = if action == "assign" {
        "role_assigned"
    } else {
        "role_revoked"
    };
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ($1, $2, 'accepted', $3)",
            &[&event_type, &user_id, &json!({ "actor_id": principal.subject, "merchant_id": principal.merchant_id, "role": role, "action": action, "mutation_id": mutation_id })],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction.commit().await.map_err(ServiceError::Database)?;
    Ok(axum::Json(RoleMutationResponse {
        schema: "auth-role-mutation/v1",
        mutation_id,
        idempotent_replay: false,
        user_id,
        role,
        action: action.to_string(),
        status: "accepted".to_string(),
    }))
}

async fn assign_role(
    State(state): State<AppState>,
    path: Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    mutate_role(State(state), path, headers, "assign").await
}

async fn revoke_role(
    State(state): State<AppState>,
    path: Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ServiceError> {
    mutate_role(State(state), path, headers, "revoke").await
}

async fn request_email(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    axum::Json(request): axum::Json<EmailRequest>,
) -> Result<impl IntoResponse, ServiceError> {
    let _permit = state
        .workers
        .acquire()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let email = normalize_email(&request.email).map_err(|_| ServiceError::BadRequest)?;
    let intent =
        parse_auth_intent(request.intent.as_deref()).map_err(|_| ServiceError::BadRequest)?;
    let now = now_unix()?;
    let source = source_digest(&state.otp_pepper, &peer.ip().to_string());
    let challenge_id = Uuid::now_v7();
    let code = generate_otp().map_err(|_| ServiceError::Unavailable)?;
    let digest = otp_digest(&state.otp_pepper, challenge_id, &email, &code).to_vec();
    let expires_at = SystemTime::now()
        .checked_add(Duration::from_secs(state.policy.ttl_seconds as u64))
        .ok_or(ServiceError::Unavailable)?;
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    let invitation_role = if let Some(invitation_id) = request.invitation_id {
        if intent != AuthIntent::SignUp {
            return Err(ServiceError::BadRequest);
        }
        let invitation = transaction
            .query_opt(
                "SELECT role, status, expires_at FROM auth_tenant_invitations WHERE id = $1 AND email_normalized = $2 FOR UPDATE",
                &[&invitation_id, &email],
            )
            .await
            .map_err(ServiceError::Database)?;
        let Some(invitation) = invitation else {
            return Err(ServiceError::AuthenticationFailed);
        };
        let status: String = invitation.get(1);
        let expires_at: SystemTime = invitation.get(2);
        if status != "pending" || SystemTime::now() >= expires_at {
            return Err(ServiceError::AuthenticationFailed);
        }
        Some(invitation.get::<_, String>(0))
    } else {
        None
    };
    let email_count: i64 = transaction
        .query_one(
            "SELECT count(*) FROM auth_otp_challenges WHERE email_normalized = $1 AND created_at >= now() - ($2::bigint * interval '1 second')",
            &[&email, &state.policy.rate_limit_window_seconds],
        )
        .await
        .map_err(ServiceError::Database)?
        .get(0);
    let source_count: i64 = transaction
        .query_one(
            "SELECT count(*) FROM auth_otp_challenges WHERE source_digest = $1 AND created_at >= now() - ($2::bigint * interval '1 second')",
            &[&source, &state.policy.rate_limit_window_seconds],
        )
        .await
        .map_err(ServiceError::Database)?
        .get(0);
    let latest_email: Option<SystemTime> = transaction
        .query_one(
            "SELECT max(created_at) FROM auth_otp_challenges WHERE email_normalized = $1 AND created_at >= now() - ($2::bigint * interval '1 second')",
            &[&email, &state.policy.rate_limit_window_seconds],
        )
        .await
        .map_err(ServiceError::Database)?
        .get(0);
    let email_limited = email_count >= i64::from(state.policy.max_email_requests);
    let source_limited = source_count >= i64::from(state.policy.max_source_requests);
    let cooldown_limited = latest_email.is_some_and(|created| {
        SystemTime::now()
            .duration_since(created)
            .map(|elapsed| elapsed.as_secs() < state.policy.resend_cooldown_seconds as u64)
            .unwrap_or(true)
    });
    if email_limited || source_limited || cooldown_limited {
        warn!(
            email_count,
            source_count,
            max_email_requests = state.policy.max_email_requests,
            max_source_requests = state.policy.max_source_requests,
            email_limited,
            source_limited,
            cooldown_limited,
            "email OTP request rate limited"
        );
        return Err(ServiceError::RateLimited);
    }
    transaction
        .execute(
            "INSERT INTO auth_otp_challenges (id, email_normalized, code_digest, source_digest, invitation_id, intent, status, max_attempts, expires_at) VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8)",
            &[&challenge_id, &email, &digest, &source, &request.invitation_id, &match intent { AuthIntent::SignIn => "signin", AuthIntent::SignUp => "signup" }, &state.policy.max_attempts, &expires_at],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, challenge_id, outcome, metadata) VALUES ('otp_requested', $1, 'accepted', $2)",
            &[&challenge_id, &json!({ "email_domain": email.rsplit('@').next().unwrap_or("unknown") })],
        )
        .await
        .map_err(ServiceError::Database)?;
    transaction.commit().await.map_err(ServiceError::Database)?;

    let message = EmailMessage {
        challenge_id,
        recipient: email.clone(),
        code,
        expires_at_unix: now.saturating_add(state.policy.ttl_seconds),
    };
    if state.email_sender.send(message).await.is_err() {
        let client = state
            .pool
            .get()
            .await
            .map_err(|_| ServiceError::Unavailable)?;
        client
            .execute(
                "UPDATE auth_otp_challenges SET status = 'delivery_failed' WHERE id = $1",
                &[&challenge_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        client
            .execute(
                "INSERT INTO auth_audit_events (event_type, challenge_id, outcome) VALUES ('otp_delivery_failed', $1, 'failed')",
                &[&challenge_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        return Err(ServiceError::EmailDelivery);
    }
    let client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    client
        .execute(
            "UPDATE auth_otp_challenges SET status = 'sent', delivered_at = now() WHERE id = $1",
            &[&challenge_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    client
        .execute(
            "INSERT INTO auth_audit_events (event_type, challenge_id, outcome) VALUES ('otp_delivered', $1, 'accepted')",
            &[&challenge_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    info!(challenge_id = %challenge_id, "email OTP delivered");
    Ok((
        StatusCode::ACCEPTED,
        axum::Json(ChallengeResponse {
            challenge_id,
            expires_at_unix: now.saturating_add(state.policy.ttl_seconds),
            retry_after_seconds: state.policy.resend_cooldown_seconds,
            invitation_role,
        }),
    ))
}

fn initial_admin_bootstrap_allowed(
    configured_email: Option<&str>,
    requested_email: &str,
    has_active_admin: bool,
) -> bool {
    configured_email.is_some_and(|email| email == requested_email) && !has_active_admin
}

fn signup_provisioning_allowed(
    self_signup: bool,
    has_pending_invitation: bool,
    initial_admin_candidate: bool,
) -> bool {
    self_signup || has_pending_invitation || initial_admin_candidate
}

fn tenant_owner_candidate(open_self_signup: bool, initial_admin_candidate: bool) -> bool {
    open_self_signup || initial_admin_candidate
}

fn account_flow_error(
    intent: AuthIntent,
    has_existing_user: bool,
    initial_admin_email_matches: bool,
) -> Option<ServiceError> {
    match (intent, has_existing_user, initial_admin_email_matches) {
        (AuthIntent::SignIn, false, true) => Some(ServiceError::AccountSetupRequired),
        (AuthIntent::SignIn, false, false) => Some(ServiceError::AuthenticationFailed),
        (AuthIntent::SignUp, true, _) => Some(ServiceError::AccountExists),
        _ => None,
    }
}

async fn consume_verified_challenge(
    transaction: &tokio_postgres::Transaction<'_>,
    challenge_id: Uuid,
    subject: Option<Uuid>,
    terminal_outcome: &str,
) -> Result<(), ServiceError> {
    transaction
        .execute(
            "UPDATE auth_otp_challenges SET status = 'consumed', consumed_at = now() WHERE id = $1 AND status = 'sent'",
            &[&challenge_id],
        )
        .await
        .map_err(ServiceError::Database)?;
    let metadata = json!({ "terminal_outcome": terminal_outcome });
    transaction
        .execute(
            "INSERT INTO auth_audit_events (event_type, subject_id, challenge_id, outcome, metadata) VALUES ('otp_verified', $1, $2, 'accepted', $3)",
            &[&subject, &challenge_id, &metadata],
        )
        .await
        .map_err(ServiceError::Database)?;
    Ok(())
}

async fn verify_email(
    State(state): State<AppState>,
    axum::Json(request): axum::Json<EmailVerificationRequest>,
) -> Result<impl IntoResponse, ServiceError> {
    let _permit = state
        .workers
        .acquire()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let email = normalize_email(&request.email).map_err(|_| ServiceError::AuthenticationFailed)?;
    validate_otp_code(&request.code).map_err(|_| ServiceError::AuthenticationFailed)?;
    let mut client = state
        .pool
        .get()
        .await
        .map_err(|_| ServiceError::Unavailable)?;
    let transaction = client.transaction().await.map_err(ServiceError::Database)?;
    transaction
        .query_one(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 441))",
            &[&email],
        )
        .await
        .map_err(ServiceError::Database)?;
    let row = transaction
        .query_opt(
            "SELECT email_normalized, code_digest, intent, status, attempts, max_attempts, expires_at, invitation_id FROM auth_otp_challenges WHERE id = $1 FOR UPDATE",
            &[&request.challenge_id],
        )
        .await
        .map_err(ServiceError::Database)?
        .ok_or(ServiceError::AuthenticationFailed)?;
    let stored_email: String = row.get(0);
    let stored_digest: Vec<u8> = row.get(1);
    let stored_intent: String = row.get(2);
    let status: String = row.get(3);
    let attempts: i32 = row.get(4);
    let max_attempts: i32 = row.get(5);
    let expires_at: SystemTime = row.get(6);
    let bound_invitation_id: Option<Uuid> = row.get(7);
    if stored_email != email || status != "sent" || SystemTime::now() >= expires_at {
        transaction
            .execute("UPDATE auth_otp_challenges SET status = CASE WHEN status = 'sent' THEN 'expired' ELSE status END WHERE id = $1", &[&request.challenge_id])
            .await
            .map_err(ServiceError::Database)?;
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Err(ServiceError::AuthenticationFailed);
    }
    let candidate = otp_digest(
        &state.otp_pepper,
        request.challenge_id,
        &email,
        &request.code,
    );
    let digest_matches = stored_digest.len() == 32
        && constant_time_digest_eq(
            &candidate,
            stored_digest
                .as_slice()
                .try_into()
                .map_err(|_| ServiceError::AuthenticationFailed)?,
        );
    if !digest_matches {
        let next_attempts = attempts.saturating_add(1);
        let next_status = if next_attempts >= max_attempts {
            "exhausted"
        } else {
            "sent"
        };
        transaction
            .execute(
                "UPDATE auth_otp_challenges SET attempts = $1, status = $2 WHERE id = $3",
                &[&next_attempts, &next_status, &request.challenge_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction
            .execute("INSERT INTO auth_audit_events (event_type, challenge_id, outcome) VALUES ('otp_rejected', $1, 'rejected')", &[&request.challenge_id])
            .await
            .map_err(ServiceError::Database)?;
        transaction.commit().await.map_err(ServiceError::Database)?;
        return Err(ServiceError::AuthenticationFailed);
    }

    let existing = transaction
        .query_opt(
            "SELECT id, merchant_id, status FROM auth_users WHERE email_normalized = $1 FOR UPDATE",
            &[&email],
        )
        .await
        .map_err(ServiceError::Database)?;
    let requested_intent = match stored_intent.as_str() {
        "signin" => AuthIntent::SignIn,
        "signup" => AuthIntent::SignUp,
        _ => {
            consume_verified_challenge(&transaction, request.challenge_id, None, "invalid_intent")
                .await?;
            transaction.commit().await.map_err(ServiceError::Database)?;
            return Err(ServiceError::AuthenticationFailed);
        }
    };
    let pending_invitation = if requested_intent == AuthIntent::SignUp {
        let invitation = if let Some(invitation_id) = bound_invitation_id {
            transaction
                .query_opt(
                    "SELECT id, merchant_id, role, expires_at, invited_by FROM auth_tenant_invitations WHERE id = $1 AND email_normalized = $2 AND status = 'pending' FOR UPDATE",
                    &[&invitation_id, &email],
                )
                .await
                .map_err(ServiceError::Database)?
        } else {
            transaction
                .query_opt(
                    "SELECT id, merchant_id, role, expires_at, invited_by FROM auth_tenant_invitations WHERE email_normalized = $1 AND status = 'pending' ORDER BY created_at DESC, id DESC LIMIT 1 FOR UPDATE",
                    &[&email],
                )
                .await
                .map_err(ServiceError::Database)?
        };
        if bound_invitation_id.is_some() && invitation.is_none() {
            consume_verified_challenge(
                &transaction,
                request.challenge_id,
                None,
                "invitation_unavailable",
            )
            .await?;
            transaction.commit().await.map_err(ServiceError::Database)?;
            return Err(ServiceError::AuthenticationFailed);
        }
        if let Some(invitation) = invitation {
            let expires_at: SystemTime = invitation.get(3);
            if SystemTime::now() >= expires_at {
                transaction
                    .execute(
                        "UPDATE auth_tenant_invitations SET status = 'expired' WHERE id = $1",
                        &[&invitation.get::<_, Uuid>(0)],
                    )
                    .await
                    .map_err(ServiceError::Database)?;
                None
            } else {
                Some((
                    invitation.get::<_, Uuid>(0),
                    invitation.get::<_, Uuid>(1),
                    invitation.get::<_, String>(2),
                    invitation.get::<_, Uuid>(4),
                ))
            }
        } else {
            None
        }
    } else {
        None
    };
    let initial_admin_candidate = if requested_intent == AuthIntent::SignUp {
        let has_active_admin = transaction
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM auth_user_roles WHERE role = 'administrator' AND status = 'active')",
                &[],
            )
            .await
            .map_err(ServiceError::Database)?
            .get(0);
        initial_admin_bootstrap_allowed(
            state.initial_admin_email.as_deref(),
            &email,
            has_active_admin,
        )
    } else {
        false
    };
    let (subject, merchant_id, provisioned, accepted_invitation, provisioning_mode) = if let Some(
        user,
    ) =
        existing
    {
        if let Some(error) = account_flow_error(requested_intent, true, false) {
            let terminal_outcome = if matches!(&error, ServiceError::AccountExists) {
                "account_exists"
            } else {
                "account_setup_required"
            };
            let subject: Uuid = user.get(0);
            consume_verified_challenge(
                &transaction,
                request.challenge_id,
                Some(subject),
                terminal_outcome,
            )
            .await?;
            transaction.commit().await.map_err(ServiceError::Database)?;
            return Err(error);
        }
        let status: String = user.get(2);
        if status != "active" {
            let subject: Uuid = user.get(0);
            consume_verified_challenge(
                &transaction,
                request.challenge_id,
                Some(subject),
                "account_inactive",
            )
            .await?;
            transaction.commit().await.map_err(ServiceError::Database)?;
            return Err(ServiceError::AuthenticationFailed);
        }
        let subject: Uuid = user.get(0);
        let merchant_id: Uuid = user.get(1);
        transaction
            .execute(
                "UPDATE auth_users SET last_verified_at = now() WHERE id = $1",
                &[&subject],
            )
            .await
            .map_err(ServiceError::Database)?;
        (subject, merchant_id, false, None, "existing")
    } else {
        if let Some(error) = account_flow_error(
            requested_intent,
            false,
            state.initial_admin_email.as_deref() == Some(email.as_str()),
        ) {
            let terminal_outcome = if matches!(&error, ServiceError::AccountSetupRequired) {
                "account_setup_required"
            } else {
                "authentication_failed"
            };
            consume_verified_challenge(&transaction, request.challenge_id, None, terminal_outcome)
                .await?;
            transaction.commit().await.map_err(ServiceError::Database)?;
            return Err(error);
        }
        if !signup_provisioning_allowed(
            state.self_signup,
            pending_invitation.is_some(),
            initial_admin_candidate,
        ) {
            consume_verified_challenge(
                &transaction,
                request.challenge_id,
                None,
                "provisioning_not_allowed",
            )
            .await?;
            transaction.commit().await.map_err(ServiceError::Database)?;
            return Err(ServiceError::AuthenticationFailed);
        }
        let open_self_signup =
            state.self_signup && pending_invitation.is_none() && !initial_admin_candidate;
        if open_self_signup {
            transaction
                .query_one(
                    "SELECT pg_advisory_xact_lock(hashtextextended($1, 522))",
                    &[&"auth:self-signup-daily"],
                )
                .await
                .map_err(ServiceError::Database)?;
            let daily_count: i64 = transaction
                .query_one(
                    "SELECT count(*) FROM auth_audit_events WHERE event_type = 'user_provisioned' AND occurred_at >= date_trunc('day', timezone('UTC', now())) AT TIME ZONE 'UTC' AND metadata->>'provisioning' = 'self_signup'",
                    &[],
                )
                .await
                .map_err(ServiceError::Database)?
                .get(0);
            if daily_count >= state.self_signup_daily_limit {
                consume_verified_challenge(
                    &transaction,
                    request.challenge_id,
                    None,
                    "self_signup_daily_limit_reached",
                )
                .await?;
                transaction.commit().await.map_err(ServiceError::Database)?;
                return Err(ServiceError::SelfSignupDailyLimitReached);
            }
        }
        let subject = Uuid::now_v7();
        let owner_candidate = tenant_owner_candidate(open_self_signup, initial_admin_candidate);
        let provisioning_mode = if pending_invitation.is_some() {
            "invitation"
        } else if initial_admin_candidate {
            "initial_admin"
        } else {
            "self_signup"
        };
        let (merchant_id, invitation) = pending_invitation
            .map(|(invitation_id, merchant_id, role, invited_by)| {
                (merchant_id, Some((invitation_id, role, invited_by)))
            })
            .unwrap_or((Uuid::now_v7(), None));
        transaction
            .execute(
                "INSERT INTO auth_tenants (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
                &[&merchant_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction
            .execute(
                "INSERT INTO auth_users (id, email_normalized, merchant_id) VALUES ($1, $2, $3)",
                &[&subject, &email, &merchant_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        if owner_candidate || state.local_bootstrap_admin_email.as_deref() == Some(email.as_str()) {
            let reason = if open_self_signup {
                "tenant_owner_self_signup"
            } else if initial_admin_candidate {
                "configured_initial_admin"
            } else {
                "local_bootstrap"
            };
            transaction
                .execute(
                    "INSERT INTO auth_user_roles (user_id, role, status, assigned_by) VALUES ($1, 'administrator', 'active', $1) ON CONFLICT (user_id, role) DO UPDATE SET status = 'active', assigned_at = now(), assigned_by = EXCLUDED.assigned_by",
                    &[&subject],
                )
                .await
                .map_err(ServiceError::Database)?;
            transaction
                .execute(
                    "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('role_assigned', $1, 'accepted', $2)",
                    &[&subject, &json!({ "role": "administrator", "reason": reason })],
                )
                .await
                .map_err(ServiceError::Database)?;
        }
        (subject, merchant_id, true, invitation, provisioning_mode)
    };
    if let Some((invitation_id, invited_role, invited_by)) = accepted_invitation {
        transaction
            .execute(
                "INSERT INTO auth_user_roles (user_id, role, status, assigned_by) VALUES ($1, $2, 'active', $3) ON CONFLICT (user_id, role) DO UPDATE SET status = 'active', assigned_at = now(), assigned_by = EXCLUDED.assigned_by",
                &[&subject, &invited_role, &invited_by],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction
            .execute(
                "UPDATE auth_tenant_invitations SET status = 'accepted', accepted_user_id = $1, accepted_at = now() WHERE id = $2",
                &[&subject, &invitation_id],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction
            .execute(
                "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('invitation_accepted', $1, 'accepted', $2)",
                &[&subject, &json!({ "invitation_id": invitation_id, "merchant_id": merchant_id, "role": invited_role, "invited_by": invited_by })],
            )
            .await
            .map_err(ServiceError::Database)?;
        transaction
            .execute(
                "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('role_assigned', $1, 'accepted', $2)",
                &[&subject, &json!({ "role": invited_role, "reason": "tenant_invitation", "invitation_id": invitation_id, "assigned_by": invited_by })],
            )
            .await
            .map_err(ServiceError::Database)?;
    }
    let assigned_roles = transaction
        .query(
            "SELECT role FROM auth_user_roles WHERE user_id = $1 AND status = 'active' ORDER BY role",
            &[&subject],
        )
        .await
        .map_err(ServiceError::Database)?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    let mut roles = state.default_roles.to_vec();
    for role in assigned_roles {
        if !roles.iter().any(|existing| existing == &role) {
            roles.push(role);
        }
    }
    consume_verified_challenge(
        &transaction,
        request.challenge_id,
        Some(subject),
        if provisioned {
            "provisioned"
        } else {
            "session_issued"
        },
    )
    .await?;
    if provisioned {
        transaction
            .execute(
                "INSERT INTO auth_audit_events (event_type, subject_id, outcome, metadata) VALUES ('user_provisioned', $1, 'accepted', $2)",
                &[&subject, &json!({ "provisioning": provisioning_mode })],
            )
            .await
            .map_err(ServiceError::Database)?;
    }
    transaction.commit().await.map_err(ServiceError::Database)?;
    let now = now_unix()?;
    let response = state
        .issuer
        .issue_tokens(TokenRequest {
            subject,
            merchant_id,
            email,
            scopes: derive_role_scopes_with_profiles(
                &state.default_scopes,
                &state.administrator_scopes,
                &state.moderator_scopes,
                &roles,
                &state.role_profiles,
            ),
            roles,
            now_unix: now,
            nonce: None,
        })
        .map_err(|_| ServiceError::Internal)?;
    Ok(axum::Json(response))
}

fn environment() -> Result<String, ServiceError> {
    let value = env::var("AUTH_ENV").unwrap_or_else(|_| "local".to_string());
    match value.as_str() {
        "local" | "staging" | "production" => Ok(value),
        _ => Err(ServiceError::Configuration(
            "AUTH_ENV must be local, staging, or production".to_string(),
        )),
    }
}

fn parse_cors_origins(raw: &str) -> Result<Vec<HeaderValue>, ServiceError> {
    let mut origins = Vec::new();
    for value in raw.split(',').map(str::trim) {
        if value.is_empty() || value == "*" {
            return Err(ServiceError::Configuration(
                "AUTH_CORS_ORIGIN must contain non-empty exact origins".to_string(),
            ));
        }
        let Some(host_and_port) = value
            .strip_prefix("http://")
            .or_else(|| value.strip_prefix("https://"))
        else {
            return Err(ServiceError::Configuration(
                "AUTH_CORS_ORIGIN entries must use http or https".to_string(),
            ));
        };
        if host_and_port.is_empty()
            || host_and_port
                .chars()
                .any(|character| matches!(character, '/' | '?' | '#' | '@' | ' ' | '\t'))
        {
            return Err(ServiceError::Configuration(
                "AUTH_CORS_ORIGIN entries must be exact origins without paths or credentials"
                    .to_string(),
            ));
        }
        let origin = HeaderValue::from_str(value).map_err(|_| {
            ServiceError::Configuration("AUTH_CORS_ORIGIN contains an invalid origin".to_string())
        })?;
        if !origins.contains(&origin) {
            origins.push(origin);
        }
    }
    if origins.is_empty() {
        return Err(ServiceError::Configuration(
            "AUTH_CORS_ORIGIN must contain at least one exact origin".to_string(),
        ));
    }
    Ok(origins)
}

fn validate_provider_value(value: &str, key: &str, max_bytes: usize) -> Result<(), ServiceError> {
    if value.trim().is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(ServiceError::Configuration(format!(
            "{key} is empty, contains control characters, or is unbounded"
        )));
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

fn validate_provider_endpoint(endpoint: &Url, environment: &str) -> Result<(), ServiceError> {
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(ServiceError::Configuration(
            "AUTH_EMAIL_PROVIDER_ENDPOINT contains credentials or mutable URL components"
                .to_string(),
        ));
    }
    let host = endpoint.host_str().ok_or_else(|| {
        ServiceError::Configuration("AUTH_EMAIL_PROVIDER_ENDPOINT has no host".to_string())
    })?;
    if environment == "local" {
        if !is_loopback_host(host) || endpoint.port().is_none() {
            return Err(ServiceError::Configuration(
                "local email provider endpoint must be an explicit loopback URL".to_string(),
            ));
        }
        if endpoint.scheme() != "http" && endpoint.scheme() != "https" {
            return Err(ServiceError::Configuration(
                "local email provider endpoint must use HTTP or HTTPS".to_string(),
            ));
        }
    } else if endpoint.scheme() != "https" {
        return Err(ServiceError::Configuration(
            "staging and production email provider endpoints require HTTPS".to_string(),
        ));
    }
    Ok(())
}

fn validate_app_base_url(value: &str, environment: &str) -> Result<String, ServiceError> {
    validate_provider_value(value, "AUTH_APP_BASE_URL", 512)?;
    let url = Url::parse(value).map_err(|_| {
        ServiceError::Configuration("AUTH_APP_BASE_URL is not a valid URL".to_string())
    })?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || (url.path() != "" && url.path() != "/")
    {
        return Err(ServiceError::Configuration(
            "AUTH_APP_BASE_URL must be an exact origin".to_string(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| ServiceError::Configuration("AUTH_APP_BASE_URL has no host".to_string()))?;
    if environment == "local" {
        if !is_loopback_host(host) {
            return Err(ServiceError::Configuration(
                "local AUTH_APP_BASE_URL must use a loopback host".to_string(),
            ));
        }
    } else if url.scheme() != "https" {
        return Err(ServiceError::Configuration(
            "staging and production AUTH_APP_BASE_URL requires HTTPS".to_string(),
        ));
    }
    Ok(value.trim_end_matches('/').to_string())
}

fn configured_provider_timeout(environment: &str) -> Result<Duration, ServiceError> {
    let raw = env::var("AUTH_EMAIL_PROVIDER_TIMEOUT_MILLIS")
        .unwrap_or_else(|_| DEFAULT_PROVIDER_TIMEOUT_MILLIS.to_string());
    let millis = raw.parse::<u64>().map_err(|_| {
        ServiceError::Configuration(
            "AUTH_EMAIL_PROVIDER_TIMEOUT_MILLIS must be a bounded integer".to_string(),
        )
    })?;
    if millis == 0 || millis > MAX_PROVIDER_TIMEOUT_MILLIS {
        return Err(ServiceError::Configuration(
            "AUTH_EMAIL_PROVIDER_TIMEOUT_MILLIS is outside its bounded range".to_string(),
        ));
    }
    if matches!(environment, "local" | "staging" | "production") {
        Ok(Duration::from_millis(millis))
    } else {
        Err(ServiceError::Configuration(
            "AUTH_ENV must be local, staging, or production".to_string(),
        ))
    }
}

fn configured_smtp_timeout(environment: &str) -> Result<Duration, ServiceError> {
    let raw = env::var("AUTH_EMAIL_SMTP_TIMEOUT_MILLIS")
        .unwrap_or_else(|_| DEFAULT_SMTP_TIMEOUT_MILLIS.to_string());
    let millis = raw.parse::<u64>().map_err(|_| {
        ServiceError::Configuration(
            "AUTH_EMAIL_SMTP_TIMEOUT_MILLIS must be a bounded integer".to_string(),
        )
    })?;
    if millis == 0 || millis > MAX_SMTP_TIMEOUT_MILLIS {
        return Err(ServiceError::Configuration(
            "AUTH_EMAIL_SMTP_TIMEOUT_MILLIS is outside its bounded range".to_string(),
        ));
    }
    if matches!(environment, "local" | "staging" | "production") {
        Ok(Duration::from_millis(millis))
    } else {
        Err(ServiceError::Configuration(
            "AUTH_ENV must be local, staging, or production".to_string(),
        ))
    }
}

fn parse_google_smtp_config(
    environment: &str,
    host: Option<&str>,
    port: Option<&str>,
    security: Option<&str>,
    authentication: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<GoogleSmtpConfig, ServiceError> {
    let authentication = match authentication.unwrap_or(DEFAULT_SMTP_AUTHENTICATION) {
        "app-password" => SmtpAuthentication::AppPassword,
        "ip-allowlist" => SmtpAuthentication::IpAllowlist,
        _ => {
            return Err(ServiceError::Configuration(
                "AUTH_EMAIL_SMTP_AUTH must be app-password or ip-allowlist".to_string(),
            ));
        }
    };
    let default_host = match authentication {
        SmtpAuthentication::AppPassword => "smtp.gmail.com",
        SmtpAuthentication::IpAllowlist => DEFAULT_SMTP_HOST,
    };
    let host = host.unwrap_or(default_host);
    validate_provider_value(host, "AUTH_EMAIL_SMTP_HOST", 253)?;
    if host.chars().any(|character| {
        character.is_whitespace() || matches!(character, '/' | '\\' | ':' | '@' | '?' | '#')
    }) {
        return Err(ServiceError::Configuration(
            "AUTH_EMAIL_SMTP_HOST must be a DNS host name".to_string(),
        ));
    }
    let port = port
        .unwrap_or(DEFAULT_SMTP_PORT)
        .parse::<u16>()
        .map_err(|_| {
            ServiceError::Configuration("AUTH_EMAIL_SMTP_PORT must be a valid port".to_string())
        })?;
    let security = match security.unwrap_or(DEFAULT_SMTP_SECURITY) {
        "starttls" => SmtpSecurity::StartTls,
        "tls" => SmtpSecurity::Tls,
        _ => {
            return Err(ServiceError::Configuration(
                "AUTH_EMAIL_SMTP_SECURITY must be starttls or tls".to_string(),
            ));
        }
    };
    match security {
        SmtpSecurity::StartTls if !matches!(port, 25 | 587) => {
            return Err(ServiceError::Configuration(
                "AUTH_EMAIL_SMTP_PORT must be 25 or 587 for starttls".to_string(),
            ));
        }
        SmtpSecurity::Tls if port != 465 => {
            return Err(ServiceError::Configuration(
                "AUTH_EMAIL_SMTP_PORT must be 465 for implicit TLS".to_string(),
            ));
        }
        _ => {}
    }
    if environment != "local" && matches!(security, SmtpSecurity::StartTls) && port == 25 {
        return Err(ServiceError::Configuration(
            "non-local Google SMTP must use authenticated submission port 587 or implicit TLS"
                .to_string(),
        ));
    }
    let username = username
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    let password = password
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    match authentication {
        SmtpAuthentication::AppPassword => {
            let username_value = username.as_deref().ok_or_else(|| {
                ServiceError::Configuration(
                    "AUTH_EMAIL_SMTP_USERNAME is required for app-password authentication"
                        .to_string(),
                )
            })?;
            validate_provider_value(username_value, "AUTH_EMAIL_SMTP_USERNAME", 320)?;
            if username_value.parse::<Mailbox>().is_err() {
                return Err(ServiceError::Configuration(
                    "AUTH_EMAIL_SMTP_USERNAME must be a valid Workspace address".to_string(),
                ));
            }
            let password_value = password.as_deref().ok_or_else(|| {
                ServiceError::Configuration(
                    "AUTH_EMAIL_SMTP_PASSWORD is required for app-password authentication"
                        .to_string(),
                )
            })?;
            validate_provider_value(password_value, "AUTH_EMAIL_SMTP_PASSWORD", 256)?;
        }
        SmtpAuthentication::IpAllowlist => {
            if username.is_some() || password.is_some() {
                return Err(ServiceError::Configuration(
                    "SMTP IP-allowlist mode must not receive username or password".to_string(),
                ));
            }
        }
    }
    Ok(GoogleSmtpConfig {
        host: host.to_string(),
        port,
        security,
        authentication,
        username: username.map(Arc::from),
        password: password.map(Arc::from),
    })
}

fn build_google_smtp_sender(environment: &str) -> Result<Arc<dyn EmailSender>, ServiceError> {
    let from = env::var("AUTH_EMAIL_FROM")
        .map_err(|_| ServiceError::Configuration("AUTH_EMAIL_FROM is required".to_string()))?;
    validate_provider_value(&from, "AUTH_EMAIL_FROM", MAX_PROVIDER_FROM_BYTES)?;
    let from = from.parse::<Mailbox>().map_err(|_| {
        ServiceError::Configuration("AUTH_EMAIL_FROM must be a valid mailbox address".to_string())
    })?;
    let app_base_url =
        env::var("AUTH_APP_BASE_URL").unwrap_or_else(|_| DEFAULT_LOCAL_APP_BASE_URL.to_string());
    let app_base_url = validate_app_base_url(&app_base_url, environment)?;
    let config = parse_google_smtp_config(
        environment,
        env::var("AUTH_EMAIL_SMTP_HOST").ok().as_deref(),
        env::var("AUTH_EMAIL_SMTP_PORT").ok().as_deref(),
        env::var("AUTH_EMAIL_SMTP_SECURITY").ok().as_deref(),
        env::var("AUTH_EMAIL_SMTP_AUTH").ok().as_deref(),
        env::var("AUTH_EMAIL_SMTP_USERNAME").ok().as_deref(),
        env::var("AUTH_EMAIL_SMTP_PASSWORD").ok().as_deref(),
    )?;
    let timeout = configured_smtp_timeout(environment)?;
    let builder = match config.security {
        SmtpSecurity::StartTls => {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host).map_err(|_| {
                ServiceError::Configuration("Google SMTP host is invalid".to_string())
            })?
        }
        SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|_| ServiceError::Configuration("Google SMTP host is invalid".to_string()))?,
    };
    let builder = builder.port(config.port).timeout(Some(timeout));
    let builder = match (config.authentication, config.username, config.password) {
        (SmtpAuthentication::AppPassword, Some(username), Some(password)) => {
            builder.credentials(Credentials::new(username.to_string(), password.to_string()))
        }
        (SmtpAuthentication::IpAllowlist, None, None) => builder,
        _ => {
            return Err(ServiceError::Configuration(
                "Google SMTP credentials do not match the selected authentication mode".to_string(),
            ));
        }
    };
    Ok(Arc::new(GoogleSmtpEmailSender {
        transport: Arc::new(builder.build::<Tokio1Executor>()),
        from,
        app_base_url: Arc::from(app_base_url),
    }))
}

fn build_email_sender(
    environment: &str,
    email_mode: &str,
    inbox_dir: &str,
) -> Result<Arc<dyn EmailSender>, ServiceError> {
    if email_mode == "local-file" {
        if environment != "local" {
            return Err(ServiceError::Configuration(
                "AUTH_EMAIL_MODE=local-file is local-only".to_string(),
            ));
        }
        return Ok(Arc::new(LocalInboxSender {
            directory: Arc::new(PathBuf::from(inbox_dir)),
        }));
    }
    if email_mode == "google-smtp" {
        return build_google_smtp_sender(environment);
    }
    if email_mode != "provider" {
        return Err(ServiceError::Configuration(
            "AUTH_EMAIL_MODE must be local-file, provider, or google-smtp".to_string(),
        ));
    }
    let endpoint = env::var("AUTH_EMAIL_PROVIDER_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_PROVIDER_ENDPOINT.to_string());
    validate_provider_value(
        &endpoint,
        "AUTH_EMAIL_PROVIDER_ENDPOINT",
        MAX_PROVIDER_ENDPOINT_BYTES,
    )?;
    let endpoint = Url::parse(&endpoint).map_err(|_| {
        ServiceError::Configuration("AUTH_EMAIL_PROVIDER_ENDPOINT is not a valid URL".to_string())
    })?;
    validate_provider_endpoint(&endpoint, environment)?;
    let api_key = env::var("AUTH_EMAIL_PROVIDER_API_KEY").map_err(|_| {
        ServiceError::Configuration("AUTH_EMAIL_PROVIDER_API_KEY is required".to_string())
    })?;
    validate_provider_value(
        &api_key,
        "AUTH_EMAIL_PROVIDER_API_KEY",
        MAX_PROVIDER_API_KEY_BYTES,
    )?;
    let from = env::var("AUTH_EMAIL_FROM")
        .map_err(|_| ServiceError::Configuration("AUTH_EMAIL_FROM is required".to_string()))?;
    validate_provider_value(&from, "AUTH_EMAIL_FROM", MAX_PROVIDER_FROM_BYTES)?;
    let app_base_url =
        env::var("AUTH_APP_BASE_URL").unwrap_or_else(|_| DEFAULT_LOCAL_APP_BASE_URL.to_string());
    let app_base_url = validate_app_base_url(&app_base_url, environment)?;
    let timeout = configured_provider_timeout(environment)?;
    let client = reqwest::Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .https_only(environment != "local")
        .build()
        .map_err(|_| ServiceError::Configuration("email provider client failed".to_string()))?;
    Ok(Arc::new(ProviderEmailSender {
        client,
        endpoint,
        api_key: Arc::from(api_key),
        from: Arc::from(from),
        app_base_url: Arc::from(app_base_url),
    }))
}

fn build_state() -> Result<(AppState, SocketAddr), ServiceError> {
    let environment = environment()?;
    let bind_address =
        env::var("AUTH_BIND_ADDRESS").unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string());
    let bind_address = bind_address
        .parse::<SocketAddr>()
        .map_err(|_| ServiceError::Configuration("AUTH_BIND_ADDRESS is invalid".to_string()))?;
    let database_url = match env::var("AUTH_DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ if environment == "local" => DEFAULT_DATABASE_URL.to_string(),
        _ => {
            return Err(ServiceError::Configuration(
                "AUTH_DATABASE_URL is required outside local".to_string(),
            ));
        }
    };
    let pool_size = env::var("AUTH_DATABASE_POOL_SIZE")
        .unwrap_or_else(|_| "8".to_string())
        .parse::<usize>()
        .map_err(|_| {
            ServiceError::Configuration("AUTH_DATABASE_POOL_SIZE is invalid".to_string())
        })?;
    if !(1..=64).contains(&pool_size) {
        return Err(ServiceError::Configuration(
            "AUTH_DATABASE_POOL_SIZE must be between 1 and 64".to_string(),
        ));
    }
    let issuer_url = match env::var("AUTH_ISSUER") {
        Ok(value) if !value.trim().is_empty() => value,
        _ if environment == "local" => DEFAULT_ISSUER.to_string(),
        _ => {
            return Err(ServiceError::Configuration(
                "AUTH_ISSUER is required outside local".to_string(),
            ));
        }
    };
    let audience = match env::var("AUTH_AUDIENCE") {
        Ok(value) if !value.trim().is_empty() => value,
        _ if environment == "local" => DEFAULT_AUDIENCE.to_string(),
        _ => {
            return Err(ServiceError::Configuration(
                "AUTH_AUDIENCE is required outside local".to_string(),
            ));
        }
    };
    let key_id = match env::var("AUTH_SIGNING_KEY_ID") {
        Ok(value) if !value.trim().is_empty() => value,
        _ if environment == "local" => "auth-local-v1".to_string(),
        _ => {
            return Err(ServiceError::Configuration(
                "AUTH_SIGNING_KEY_ID is required outside local".to_string(),
            ));
        }
    };
    let signing_key_bytes = if let Some(path) = env::var_os("AUTH_SIGNING_KEY_FILE") {
        fs::read(path).map_err(|_| {
            ServiceError::Configuration("AUTH_SIGNING_KEY_FILE could not be read".to_string())
        })?
    } else if let Ok(value) = env::var("AUTH_SIGNING_KEY_HEX")
        && !value.trim().is_empty()
    {
        value.into_bytes()
    } else if environment == "local" {
        vec![7; 32]
    } else {
        return Err(ServiceError::Configuration(
            "AUTH_SIGNING_KEY_FILE or AUTH_SIGNING_KEY_HEX is required outside local".to_string(),
        ));
    };
    let signing_key = SigningKey::from_bytes(&parse_seed(&signing_key_bytes)?);
    let issuer = Arc::new(
        TokenIssuer::new(&issuer_url, &audience, &key_id, signing_key)
            .map_err(|error| ServiceError::Configuration(error.to_string()))?,
    );
    let local_pepper = b"local-development-otp-pepper";
    let pepper = configured_bytes("AUTH_OTP_PEPPER", Some(local_pepper), &environment)?;
    if pepper.len() < 16 || pepper.len() > 256 {
        return Err(ServiceError::Configuration(
            "AUTH_OTP_PEPPER must be between 16 and 256 bytes".to_string(),
        ));
    }
    let email_mode = env::var("AUTH_EMAIL_MODE").unwrap_or_else(|_| "local-file".to_string());
    if environment != "local" && !matches!(email_mode.as_str(), "provider" | "google-smtp") {
        return Err(ServiceError::Configuration(
            "AUTH_EMAIL_MODE must be provider or google-smtp outside local".to_string(),
        ));
    }
    let inbox_dir =
        env::var("AUTH_LOCAL_INBOX_DIR").unwrap_or_else(|_| DEFAULT_LOCAL_INBOX.to_string());
    let email_sender = build_email_sender(&environment, &email_mode, &inbox_dir)?;
    let self_signup = parse_bool(
        &env::var("AUTH_SELF_SIGNUP").unwrap_or_else(|_| (environment == "local").to_string()),
        "AUTH_SELF_SIGNUP",
    )?;
    let self_signup_daily_limit = configured_self_signup_daily_limit_from(
        env::var("AUTH_SELF_SIGNUP_DAILY_LIMIT").ok().as_deref(),
        25,
        &environment,
    )?;
    let default_scopes = parse_list(
        &env::var("AUTH_DEFAULT_SCOPES").unwrap_or_else(|_| "tax:quote".to_string()),
        "AUTH_DEFAULT_SCOPES",
    )?;
    let administrator_scopes = configured_role_scopes(
        "AUTH_ADMINISTRATOR_SCOPES",
        &environment,
        "audit:read identity:role:manage",
    )?;
    let moderator_scopes =
        configured_role_scopes("AUTH_MODERATOR_SCOPES", &environment, "audit:read")?;
    let default_roles = parse_list(
        &env::var("AUTH_DEFAULT_ROLES").unwrap_or_else(|_| "merchant_operator".to_string()),
        "AUTH_DEFAULT_ROLES",
    )?;
    let (role_catalog, role_profiles) = parse_role_catalog()?;
    let bootstrap_admin_email = env::var("AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_email(&value).map(Arc::<str>::from))
        .transpose()
        .map_err(|_| {
            ServiceError::Configuration("AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL is invalid".to_string())
        })?;
    if bootstrap_admin_email.is_some() && environment != "local" {
        return Err(ServiceError::Configuration(
            "AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL is local-only".to_string(),
        ));
    }
    let initial_admin_email = env::var("AUTH_INITIAL_ADMIN_EMAIL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_email(&value).map(Arc::<str>::from))
        .transpose()
        .map_err(|_| {
            ServiceError::Configuration("AUTH_INITIAL_ADMIN_EMAIL is invalid".to_string())
        })?;
    if bootstrap_admin_email.is_some() && initial_admin_email.is_some() {
        return Err(ServiceError::Configuration(
            "AUTH_LOCAL_BOOTSTRAP_ADMIN_EMAIL and AUTH_INITIAL_ADMIN_EMAIL cannot both be configured".to_string(),
        ));
    }
    let mut otp_policy = OtpPolicy::default();
    otp_policy.rate_limit_window_seconds = configured_otp_rate_limit_window_seconds(
        &environment,
        otp_policy.rate_limit_window_seconds,
    )?;
    otp_policy.resend_cooldown_seconds = configured_local_nonnegative_seconds(
        "AUTH_OTP_RESEND_COOLDOWN_SECONDS",
        otp_policy.resend_cooldown_seconds,
        &environment,
    )?;
    let pool = build_pool(&database_url, pool_size)?;
    Ok((
        AppState {
            pool,
            issuer,
            email_sender,
            policy: otp_policy,
            otp_pepper: pepper.into(),
            self_signup,
            self_signup_daily_limit,
            default_scopes,
            administrator_scopes,
            moderator_scopes,
            default_roles,
            role_catalog,
            role_profiles,
            local_bootstrap_admin_email: bootstrap_admin_email,
            initial_admin_email,
            workers: Arc::new(Semaphore::new(32)),
            issuer_url: Arc::from(issuer_url),
            audience: Arc::from(audience),
            key_id: Arc::from(key_id),
        },
        bind_address,
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut telemetry = observability::init_tracing("auth_service")?;
    let (state, bind_address) = build_state()?;
    let cors_origins = env::var("AUTH_CORS_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:8088,http://127.0.0.1:8088".to_string());
    let origins = parse_cors_origins(&cors_origins)?;
    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/jwks.json", get(jwks))
        .route("/v1/tenant/context", get(tenant_context))
        .route("/v1/tenant/profile", post(update_tenant_profile))
        .route("/v1/admin/role-catalog", get(role_catalog))
        .route("/v1/auth/email/request", post(request_email))
        .route("/v1/auth/email/verify", post(verify_email))
        .route("/v1/admin/users", get(list_users))
        .route(
            "/v1/admin/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route(
            "/v1/admin/invitations/{invitation_id}",
            delete(revoke_invitation),
        )
        .route(
            "/v1/admin/users/{user_id}/roles/{role}",
            post(assign_role).delete(revoke_role),
        )
        .route(
            "/v1/admin/users/{user_id}/administrator",
            post(promote_administrator),
        )
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                    axum::http::header::AUTHORIZATION,
                    axum::http::HeaderName::from_static("idempotency-key"),
                ]),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    info!(address = %bind_address, issuer = %state.issuer_url, audience = %state.audience, key_id = %state.key_id, "authentication service listening");
    let result = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;
    telemetry.shutdown();
    result.map_err(|error| error.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_exact_cors_origins() {
        let origins = parse_cors_origins(" http://localhost:8088, http://127.0.0.1:8088 ")
            .expect("valid origin list");
        assert_eq!(
            origins
                .iter()
                .map(|origin| origin.to_str().expect("ASCII origin"))
                .collect::<Vec<_>>(),
            ["http://localhost:8088", "http://127.0.0.1:8088"]
        );
    }

    #[test]
    fn rejects_empty_wildcard_and_invalid_cors_origins() {
        for value in ["", " , ", "*", "not-an-origin", "http://"] {
            assert!(parse_cors_origins(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn role_catalog_is_valid_and_keeps_domain_scopes_explicit() {
        assert_eq!(ROLE_CATALOG_RESPONSE_SCHEMA, "auth-role-catalog/v1");
        let (roles, profiles) = parse_role_catalog().expect("checked-in role catalog is valid");
        assert!(
            roles
                .iter()
                .any(|role| role.id == "moderator" && role.assignable)
        );
        assert!(
            roles
                .iter()
                .any(|role| role.id == "administrator" && !role.assignable)
        );
        assert!(
            roles
                .iter()
                .any(|role| role.id == "vault_release_proposer" && role.assignable)
        );
        assert!(profiles.iter().any(|profile| {
            profile.role == "vault_release_proposer" && profile.scopes == ["tax:vault:propose"]
        }));
    }

    #[test]
    fn accepts_local_nonnegative_otp_cooldown_override() {
        assert!(matches!(
            configured_local_nonnegative_seconds_from(
                "AUTH_OTP_RESEND_COOLDOWN_SECONDS",
                Some("0"),
                60,
                "local"
            ),
            Ok(0)
        ));
    }

    #[test]
    fn accepts_five_minute_otp_rate_limit_window_for_staging() {
        assert_eq!(
            configured_otp_rate_limit_window_seconds_from(Some("300"), 900, "staging")
                .expect("five-minute staging window is valid"),
            300
        );
    }

    #[test]
    fn rejects_short_non_local_otp_rate_limit_windows() {
        assert!(
            configured_otp_rate_limit_window_seconds_from(Some("299"), 900, "production").is_err()
        );
        assert!(
            configured_otp_rate_limit_window_seconds_from(Some("299"), 900, "staging").is_err()
        );
        assert!(configured_otp_rate_limit_window_seconds_from(Some("60"), 900, "local").is_ok());
    }

    #[test]
    fn bounds_open_self_signup_daily_limit() {
        assert_eq!(
            configured_self_signup_daily_limit_from(Some("25"), 25, "staging")
                .expect("staging signup quota is valid"),
            25
        );
        assert!(configured_self_signup_daily_limit_from(Some("0"), 25, "staging").is_err());
        assert!(configured_self_signup_daily_limit_from(Some("10001"), 25, "production").is_err());
        assert_eq!(
            configured_self_signup_daily_limit_from(Some("0"), 25, "local")
                .expect("local may disable the quota"),
            0
        );
    }

    #[test]
    fn rejects_non_local_and_negative_otp_cooldown_overrides() {
        assert!(
            configured_local_nonnegative_seconds_from(
                "AUTH_OTP_RESEND_COOLDOWN_SECONDS",
                Some("0"),
                60,
                "staging"
            )
            .is_err()
        );
        assert!(
            configured_local_nonnegative_seconds_from(
                "AUTH_OTP_RESEND_COOLDOWN_SECONDS",
                Some("-1"),
                60,
                "local"
            )
            .is_err()
        );
    }

    #[test]
    fn distinguishes_initial_account_setup_and_existing_account_flows() {
        assert!(matches!(
            account_flow_error(AuthIntent::SignIn, false, true),
            Some(ServiceError::AccountSetupRequired)
        ));
        assert!(matches!(
            account_flow_error(AuthIntent::SignIn, false, false),
            Some(ServiceError::AuthenticationFailed)
        ));
        assert!(matches!(
            account_flow_error(AuthIntent::SignUp, true, false),
            Some(ServiceError::AccountExists)
        ));
        assert!(account_flow_error(AuthIntent::SignIn, true, false).is_none());
        assert!(account_flow_error(AuthIntent::SignUp, false, true).is_none());
    }

    #[test]
    fn provider_endpoint_requires_https_outside_local_and_loopback_inside_local() {
        let production = reqwest::Url::parse("http://mail.example.test/emails")
            .expect("valid provider endpoint");
        assert!(validate_provider_endpoint(&production, "production").is_err());

        let local_public = reqwest::Url::parse("http://mail.example.test:8080/emails")
            .expect("valid provider endpoint");
        assert!(validate_provider_endpoint(&local_public, "local").is_err());

        let local_loopback =
            reqwest::Url::parse("http://127.0.0.1:18086/emails").expect("valid endpoint");
        assert!(validate_provider_endpoint(&local_loopback, "local").is_ok());
    }

    #[test]
    fn provider_email_payload_escapes_untrusted_values_and_binds_idempotency() {
        let rendered =
            render_invitation_email("https://app.example.test", Uuid::nil(), "moderator<&");
        assert!(
            rendered.html.contains("moderator&lt;&amp;") && !rendered.html.contains("moderator<&")
        );
        assert!(rendered.text.contains(
            "https://app.example.test/auth/signup?invite=00000000-0000-0000-0000-000000000000"
        ));

        let payload = ProviderEmailPayload::new(
            "Protocol <auth@example.test>",
            "operator@example.test",
            "Verification code",
            rendered.text,
            rendered.html,
        );
        let json = serde_json::to_value(payload).expect("serializable provider payload");
        assert_eq!(json["to"][0], "operator@example.test");
        assert_eq!(json["from"], "Protocol <auth@example.test>");
        assert!(
            json["html"]
                .as_str()
                .is_some_and(|value| value.contains("&lt;"))
        );
    }

    #[test]
    fn provider_configuration_rejects_missing_or_control_character_values() {
        assert!(validate_provider_value("", "AUTH_EMAIL_PROVIDER_API_KEY", 4096).is_err());
        assert!(validate_provider_value("a\nkey", "AUTH_EMAIL_PROVIDER_API_KEY", 4096).is_err());
        assert!(validate_provider_value("sender@example.test", "AUTH_EMAIL_FROM", 320).is_ok());
    }

    #[test]
    fn google_smtp_app_password_configuration_requires_tls_and_credentials() {
        let default_config = parse_google_smtp_config(
            "local",
            None,
            None,
            None,
            None,
            Some("noreply@klyran.com"),
            Some("runtime-app-password"),
        )
        .expect("app-password defaults should be valid");
        assert_eq!(default_config.host, "smtp.gmail.com");

        let config = parse_google_smtp_config(
            "production",
            Some("smtp-relay.gmail.com"),
            Some("587"),
            Some("starttls"),
            Some("app-password"),
            Some("noreply@klyran.com"),
            Some("runtime-app-password"),
        )
        .expect("valid Google Workspace app-password configuration");
        assert_eq!(config.host, "smtp-relay.gmail.com");
        assert_eq!(config.port, 587);
        assert!(matches!(config.security, SmtpSecurity::StartTls));
        assert!(matches!(
            config.authentication,
            SmtpAuthentication::AppPassword
        ));
        assert_eq!(config.username.as_deref(), Some("noreply@klyran.com"));

        assert!(
            parse_google_smtp_config(
                "production",
                Some("smtp-relay.gmail.com"),
                Some("587"),
                Some("plain"),
                Some("app-password"),
                Some("noreply@klyran.com"),
                Some("runtime-app-password"),
            )
            .is_err()
        );
        assert!(
            parse_google_smtp_config(
                "production",
                Some("smtp-relay.gmail.com"),
                Some("587"),
                Some("starttls"),
                Some("app-password"),
                Some("noreply@klyran.com"),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn google_smtp_ip_relay_mode_rejects_passwords_and_plain_transport() {
        let config = parse_google_smtp_config(
            "production",
            Some("smtp-relay.gmail.com"),
            Some("587"),
            Some("starttls"),
            Some("ip-allowlist"),
            None,
            None,
        )
        .expect("valid Google Workspace IP relay configuration");
        assert!(matches!(
            config.authentication,
            SmtpAuthentication::IpAllowlist
        ));
        assert!(config.username.is_none());
        assert!(config.password.is_none());

        assert!(
            parse_google_smtp_config(
                "production",
                Some("smtp-relay.gmail.com"),
                Some("587"),
                Some("starttls"),
                Some("ip-allowlist"),
                Some("noreply@klyran.com"),
                Some("must-not-be-present"),
            )
            .is_err()
        );
        assert!(
            parse_google_smtp_config(
                "production",
                Some("smtp-relay.gmail.com"),
                Some("25"),
                Some("plain"),
                Some("ip-allowlist"),
                None,
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn google_smtp_message_preserves_plain_text_and_html_fallbacks() {
        let from = "Solana Tax Protocol <noreply@klyran.com>"
            .parse::<Mailbox>()
            .expect("valid sender mailbox");
        let message = build_smtp_message(
            &from,
            "operator@example.test",
            "Verification code",
            render_otp_email("123456"),
        )
        .expect("valid SMTP message");
        let formatted = String::from_utf8(message.formatted()).expect("UTF-8 message fixture");
        assert!(formatted.contains("noreply@klyran.com"));
        assert!(formatted.contains("operator@example.test"));
        assert!(formatted.contains("123456"));
        assert!(formatted.contains("multipart/alternative"));
        assert!(formatted.contains("text/html"));
    }

    #[test]
    fn invitation_is_the_only_signup_escape_when_open_signup_is_disabled() {
        assert!(signup_provisioning_allowed(true, false, false));
        assert!(signup_provisioning_allowed(false, true, false));
        assert!(signup_provisioning_allowed(false, false, true));
        assert!(!signup_provisioning_allowed(false, false, false));
    }

    #[test]
    fn configured_initial_admin_is_one_time_and_email_bound() {
        assert!(initial_admin_bootstrap_allowed(
            Some("admin@example.com"),
            "admin@example.com",
            false
        ));
        assert!(!initial_admin_bootstrap_allowed(
            Some("admin@example.com"),
            "other@example.com",
            false
        ));
        assert!(!initial_admin_bootstrap_allowed(
            Some("admin@example.com"),
            "admin@example.com",
            true
        ));
        assert!(!initial_admin_bootstrap_allowed(
            None,
            "admin@example.com",
            false
        ));
    }

    #[test]
    fn tenant_display_name_is_bounded_and_normalized_without_control_data() {
        assert_eq!(
            validate_tenant_display_name("  Klyran operations  ").expect("valid name"),
            "Klyran operations"
        );
        assert!(validate_tenant_display_name(" ").is_err());
        assert!(validate_tenant_display_name("bad\nname").is_err());
        assert!(validate_tenant_display_name(&"x".repeat(121)).is_err());
    }

    #[test]
    fn self_signup_is_a_tenant_owner_path_but_invitations_are_not() {
        assert!(tenant_owner_candidate(true, false));
        assert!(tenant_owner_candidate(false, true));
        assert!(!tenant_owner_candidate(false, false));
    }

    #[tokio::test]
    async fn provider_sender_posts_idempotent_json_to_local_fixture() {
        let received = Arc::new(tokio::sync::Mutex::new(None::<(String, String)>));
        let capture = Arc::clone(&received);
        let app = Router::new().route(
            "/emails",
            post(move |headers: axum::http::HeaderMap, body: String| {
                let capture = Arc::clone(&capture);
                async move {
                    let idempotency_key = headers
                        .get("idempotency-key")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                        .to_string();
                    *capture.lock().await = Some((idempotency_key, body));
                    (
                        StatusCode::OK,
                        axum::Json(json!({ "id": "fixture-email-1" })),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture listener");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("fixture server");
        });
        let sender = ProviderEmailSender {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("fixture client"),
            endpoint: Url::parse(&format!("http://{address}/emails")).expect("fixture URL"),
            api_key: Arc::from("test-provider-key"),
            from: Arc::from("Protocol <auth@example.test>"),
            app_base_url: Arc::from("http://localhost:8088"),
        };
        sender
            .send(EmailMessage {
                challenge_id: Uuid::nil(),
                recipient: "operator@example.test".to_string(),
                code: "123456".to_string(),
                expires_at_unix: 1,
            })
            .await
            .expect("provider fixture accepts email");
        let (idempotency_key, body) = received
            .lock()
            .await
            .take()
            .expect("fixture received request");
        assert_eq!(
            idempotency_key,
            "auth-otp-00000000-0000-0000-0000-000000000000"
        );
        let payload: serde_json::Value = serde_json::from_str(&body).expect("provider JSON");
        assert_eq!(payload["to"][0], "operator@example.test");
        assert_eq!(payload["from"], "Protocol <auth@example.test>");
        assert!(
            payload["html"]
                .as_str()
                .is_some_and(|html| html.contains("123456"))
        );
        server.abort();
        let _ = server.await;
    }
}
