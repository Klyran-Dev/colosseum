#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use std::{
    env,
    error::Error,
    fs::File,
    future::Future,
    io::{self, BufReader},
    net::SocketAddr,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use axum_server::{Handle, tls_rustls::RustlsConfig};
use rustls::{
    CertificateError, DigitallySignedStruct, DistinguishedName, RootCertStore, ServerConfig,
    SignatureScheme,
    client::danger::HandshakeSignatureValid,
    server::{
        WebPkiClientVerifier,
        danger::{ClientCertVerified, ClientCertVerifier},
    },
};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use tax_engine_service::{
    DEFAULT_CACHE_CAPACITY, DEFAULT_MAX_CONCURRENCY, EngineService, MAX_CACHE_CAPACITY,
    MAX_CONCURRENCY,
};
use tracing::{info, warn};

const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(10);
const MAX_ALLOWED_CLIENT_URIS: usize = 32;
const MAX_ALLOWED_CLIENT_URI_LENGTH: usize = 256;
const MAX_ALLOWED_CLIENT_URIS_JSON_LENGTH: usize = 8 * 1024;
const CLIENT_IDENTITY_SCHEME: &str = "spiffe://";
#[cfg(test)]
const TEST_CLIENT_URI: &str = "spiffe://solana-tax-protocol/tax-engine-client";

type ServiceResult = Result<(), Box<dyn Error + Send + Sync>>;
type ServerFuture = Pin<Box<dyn Future<Output = io::Result<()>> + Send>>;
type ReloadFuture = Pin<Box<dyn Future<Output = ServiceResult> + Send>>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Config {
    environment: Environment,
    bind_address: SocketAddr,
    auth_token: String,
    cache_capacity: usize,
    max_concurrency: usize,
    tls: Option<TlsFiles>,
    allowed_client_uris: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TlsFiles {
    certificate: PathBuf,
    private_key: PathBuf,
    client_ca: PathBuf,
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
    #[cfg(test)]
    fn from_values(
        environment: &str,
        bind_address: &str,
        auth_token: &str,
        cache_capacity: &str,
        max_concurrency: &str,
        tls_files: (Option<&str>, Option<&str>, Option<&str>),
    ) -> Result<Self, String> {
        Self::from_values_with_identity(
            environment,
            bind_address,
            auth_token,
            cache_capacity,
            max_concurrency,
            tls_files,
            None,
        )
    }

    fn from_values_with_identity(
        environment: &str,
        bind_address: &str,
        auth_token: &str,
        cache_capacity: &str,
        max_concurrency: &str,
        tls_files: (Option<&str>, Option<&str>, Option<&str>),
        allowed_client_uris: Option<&str>,
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
        let tls = TlsFiles::from_values(tls_files.0, tls_files.1, tls_files.2)?;
        if environment != Environment::Local && tls.is_none() {
            return Err(
                "TAX_ENGINE_TLS_CERT_FILE, TAX_ENGINE_TLS_KEY_FILE, and TAX_ENGINE_MTLS_CA_FILE are required outside local"
                    .to_string(),
            );
        }
        if environment == Environment::Local && tls.is_some() {
            return Err(
                "Tax Engine TLS files are only enabled for staging or production".to_string(),
            );
        }
        let allowed_client_uris = parse_allowed_client_uris(environment, allowed_client_uris)?;
        Ok(Self {
            environment,
            bind_address: bind_address.parse().map_err(|_| {
                "TAX_ENGINE_BIND_ADDRESS must be a valid socket address".to_string()
            })?,
            auth_token: auth_token.to_string(),
            cache_capacity,
            max_concurrency,
            tls,
            allowed_client_uris,
        })
    }

    fn from_env() -> Result<Self, String> {
        let tls_certificate = env::var("TAX_ENGINE_TLS_CERT_FILE").ok();
        let tls_private_key = env::var("TAX_ENGINE_TLS_KEY_FILE").ok();
        let tls_client_ca = env::var("TAX_ENGINE_MTLS_CA_FILE").ok();
        let allowed_client_uris = env::var("TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS").ok();
        Self::from_values_with_identity(
            &required_env("TAX_ENGINE_ENV")?,
            &env::var("TAX_ENGINE_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8084".to_string()),
            &required_env("TAX_ENGINE_AUTH_TOKEN")?,
            &env::var("TAX_ENGINE_CACHE_CAPACITY")
                .unwrap_or_else(|_| DEFAULT_CACHE_CAPACITY.to_string()),
            &env::var("TAX_ENGINE_MAX_CONCURRENCY")
                .unwrap_or_else(|_| DEFAULT_MAX_CONCURRENCY.to_string()),
            (
                tls_certificate.as_deref(),
                tls_private_key.as_deref(),
                tls_client_ca.as_deref(),
            ),
            allowed_client_uris.as_deref(),
        )
    }
}

fn parse_allowed_client_uris(
    environment: Environment,
    raw: Option<&str>,
) -> Result<Vec<String>, String> {
    match (environment, raw) {
        (Environment::Local, None) => Ok(Vec::new()),
        (Environment::Local, Some(value)) if value.trim().is_empty() => Ok(Vec::new()),
        (Environment::Local, Some(_)) => {
            Err("TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS is only valid outside local".to_string())
        }
        (_, None) => {
            Err("TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS is required outside local".to_string())
        }
        (_, Some(value)) => {
            if value.len() > MAX_ALLOWED_CLIENT_URIS_JSON_LENGTH {
                return Err(format!(
                    "TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS must be at most {MAX_ALLOWED_CLIENT_URIS_JSON_LENGTH} bytes"
                ));
            }
            let uris = serde_json::from_str::<Vec<String>>(value).map_err(|_| {
                "TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS must be a JSON array of strings".to_string()
            })?;
            if uris.is_empty() || uris.len() > MAX_ALLOWED_CLIENT_URIS {
                return Err(format!(
                    "TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS must contain 1 to {MAX_ALLOWED_CLIENT_URIS} identities"
                ));
            }
            for uri in &uris {
                if uri.is_empty()
                    || uri.len() > MAX_ALLOWED_CLIENT_URI_LENGTH
                    || !uri.is_ascii()
                    || !uri.starts_with(CLIENT_IDENTITY_SCHEME)
                    || uri
                        .bytes()
                        .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
                {
                    return Err(
                        "TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS contains an invalid identity"
                            .to_string(),
                    );
                }
            }
            let mut unique = uris.clone();
            unique.sort_unstable();
            unique.dedup();
            if unique.len() != uris.len() {
                return Err(
                    "TAX_ENGINE_MTLS_ALLOWED_CLIENT_URIS must not contain duplicates".to_string(),
                );
            }
            Ok(uris)
        }
    }
}

impl TlsFiles {
    fn from_values(
        certificate: Option<&str>,
        private_key: Option<&str>,
        client_ca: Option<&str>,
    ) -> Result<Option<Self>, String> {
        match (certificate, private_key, client_ca) {
            (None, None, None) => Ok(None),
            (Some(certificate), Some(private_key), Some(client_ca)) => {
                let certificate = non_empty_path("TAX_ENGINE_TLS_CERT_FILE", certificate)?;
                let private_key = non_empty_path("TAX_ENGINE_TLS_KEY_FILE", private_key)?;
                let client_ca = non_empty_path("TAX_ENGINE_MTLS_CA_FILE", client_ca)?;
                Ok(Some(Self {
                    certificate,
                    private_key,
                    client_ca,
                }))
            }
            _ => Err(
                "TAX_ENGINE_TLS_CERT_FILE, TAX_ENGINE_TLS_KEY_FILE, and TAX_ENGINE_MTLS_CA_FILE must be configured together"
                    .to_string(),
            ),
        }
    }
}

fn non_empty_path(name: &str, value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(PathBuf::from(value))
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
    ensure_crypto_provider()?;
    let mut telemetry = observability::init_tracing("tax_engine")?;
    let config = Config::from_env()?;
    let service = EngineService::with_limits(config.cache_capacity, config.max_concurrency)?;
    let handle = Handle::<SocketAddr>::new();
    let router = service.router(&config.auth_token);
    info!(
        address = %config.bind_address,
        environment = ?config.environment,
        cache_capacity = config.cache_capacity,
        max_concurrency = config.max_concurrency,
        transport = if config.tls.is_some() { "rustls-mtls" } else { "plain-local" },
        "tax engine service listening"
    );

    let mut tls_reload: Option<ReloadFuture> = None;
    let server: ServerFuture = match config.tls {
        Some(tls_files) => {
            let tls_config = build_mtls_config(&tls_files, &config.allowed_client_uris)?;
            tls_reload = Some(build_tls_reload_future(
                tls_config.clone(),
                tls_files,
                config.allowed_client_uris.clone(),
            ));
            Box::pin(
                axum_server::bind_rustls(config.bind_address, tls_config)
                    .handle(handle.clone())
                    .serve(router.into_make_service()),
            )
        }
        None => Box::pin(
            axum_server::bind(config.bind_address)
                .handle(handle.clone())
                .serve(router.into_make_service()),
        ),
    };
    run_server_until_shutdown(server, handle, tls_reload).await?;
    telemetry.shutdown();
    Ok(())
}

fn read_certificates(path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    rustls_pemfile::certs(&mut BufReader::new(file))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: invalid certificate PEM: {error}", path.display()))
}

fn read_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    rustls_pemfile::private_key(&mut BufReader::new(file))
        .map_err(|error| format!("{}: invalid private-key PEM: {error}", path.display()))?
        .ok_or_else(|| format!("{}: no private key found", path.display()))
}

fn ensure_crypto_provider() -> Result<(), String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        Ok(())
    } else {
        Err("no Rustls cryptographic provider is available".to_string())
    }
}

#[derive(Debug)]
struct AuthorizedClientCertVerifier {
    inner: Arc<dyn ClientCertVerifier>,
    allowed_uris: Arc<[String]>,
}

impl ClientCertVerifier for AuthorizedClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        self.inner.offer_client_auth()
    }

    fn client_auth_mandatory(&self) -> bool {
        self.inner.client_auth_mandatory()
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        self.inner.root_hint_subjects()
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.inner
            .verify_client_cert(end_entity, intermediates, now)?;
        let (_, certificate) = x509_parser::parse_x509_certificate(end_entity.as_ref())
            .map_err(|_| rustls::Error::InvalidCertificate(CertificateError::BadEncoding))?;
        let san = certificate
            .subject_alternative_name()
            .map_err(|_| rustls::Error::InvalidCertificate(CertificateError::BadEncoding))?;
        let uri_sans = san
            .into_iter()
            .flat_map(|extension| extension.value.general_names.iter())
            .filter_map(|name| match name {
                x509_parser::extensions::GeneralName::URI(uri) => Some(*uri),
                _ => None,
            })
            .collect::<Vec<_>>();
        if uri_sans.len() != 1
            || !self
                .allowed_uris
                .iter()
                .any(|allowed| allowed == uri_sans[0])
        {
            return Err(rustls::Error::General(
                "client certificate identity is not authorized".to_string(),
            ));
        }
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

fn build_mtls_server_config(
    files: &TlsFiles,
    allowed_client_uris: &[String],
) -> Result<ServerConfig, String> {
    ensure_crypto_provider()?;
    if allowed_client_uris.is_empty() {
        return Err("Tax Engine mTLS client identity allow-list is empty".to_string());
    }
    let certificates = read_certificates(&files.certificate)?;
    if certificates.is_empty() {
        return Err(format!(
            "{}: server certificate chain is empty",
            files.certificate.display()
        ));
    }
    let private_key = read_private_key(&files.private_key)?;
    let client_ca = read_certificates(&files.client_ca)?;
    if client_ca.is_empty() {
        return Err(format!(
            "{}: client CA bundle is empty",
            files.client_ca.display()
        ));
    }
    let mut roots = RootCertStore::empty();
    for certificate in client_ca {
        roots.add(certificate).map_err(|error| {
            format!("{}: invalid client CA: {error}", files.client_ca.display())
        })?;
    }
    let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|error| format!("client CA verifier could not be built: {error}"))?;
    let verifier = Arc::new(AuthorizedClientCertVerifier {
        inner: verifier,
        allowed_uris: Arc::from(allowed_client_uris.to_vec()),
    });
    let mut server_config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certificates, private_key)
        .map_err(|error| format!("server certificate/key could not be loaded: {error}"))?;
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(server_config)
}

fn build_mtls_config(
    files: &TlsFiles,
    allowed_client_uris: &[String],
) -> Result<RustlsConfig, String> {
    Ok(RustlsConfig::from_config(Arc::new(
        build_mtls_server_config(files, allowed_client_uris)?,
    )))
}

fn install_mtls_config(config: &RustlsConfig, replacement: ServerConfig) {
    config.reload_from_config(Arc::new(replacement));
}

#[cfg(test)]
fn reload_mtls_config(
    config: &RustlsConfig,
    files: &TlsFiles,
    allowed_client_uris: &[String],
) -> Result<(), String> {
    let replacement = build_mtls_server_config(files, allowed_client_uris)?;
    install_mtls_config(config, replacement);
    Ok(())
}

async fn reload_mtls_config_async(
    config: RustlsConfig,
    files: TlsFiles,
    allowed_client_uris: Vec<String>,
) -> Result<(), String> {
    let replacement =
        tokio::task::spawn_blocking(move || build_mtls_server_config(&files, &allowed_client_uris))
            .await
            .map_err(|error| format!("mTLS configuration validation task failed: {error}"))??;
    install_mtls_config(&config, replacement);
    Ok(())
}

fn build_tls_reload_future(
    config: RustlsConfig,
    files: TlsFiles,
    allowed_client_uris: Vec<String>,
) -> ReloadFuture {
    #[cfg(unix)]
    {
        Box::pin(async move {
            let mut hangup =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;
            loop {
                if hangup.recv().await.is_none() {
                    return Err("SIGHUP stream ended before shutdown".into());
                }
                match reload_mtls_config_async(
                    config.clone(),
                    files.clone(),
                    allowed_client_uris.clone(),
                )
                .await
                {
                    Ok(()) => info!("Tax Engine mTLS configuration reloaded"),
                    Err(error) => {
                        warn!(%error, "Tax Engine mTLS reload rejected; active configuration retained")
                    }
                }
            }
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (config, files, allowed_client_uris);
        Box::pin(async { Err("mTLS certificate reload requires a Unix SIGHUP signal".into()) })
    }
}

async fn run_server_until_shutdown(
    server: ServerFuture,
    handle: Handle<SocketAddr>,
    tls_reload: Option<ReloadFuture>,
) -> ServiceResult {
    tokio::pin!(server);
    let mut shutdown = Box::pin(tokio::signal::ctrl_c());
    let tls_reload = tls_reload
        .unwrap_or_else(|| Box::pin(std::future::pending::<ServiceResult>()) as ReloadFuture);
    tokio::pin!(tls_reload);
    tokio::select! {
        result = &mut server => {
            result?;
            Err("tax engine server ended before a shutdown signal was recorded".into())
        }
        shutdown_result = &mut shutdown => {
            shutdown_result?;
            handle.graceful_shutdown(Some(SHUTDOWN_GRACE_PERIOD));
            server.await?;
            Ok(())
        }
        reload_result = &mut tls_reload => {
            reload_result?;
            Err("Tax Engine mTLS reload task ended before shutdown".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use rcgen::{
        BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
        SanType,
    };
    use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConnection};
    use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};

    use super::{
        Config, Environment, TEST_CLIENT_URI, TlsFiles, build_mtls_config,
        build_mtls_server_config, reload_mtls_config, validate_required,
    };

    struct MemoryPipe {
        bytes: Vec<u8>,
    }

    impl MemoryPipe {
        fn is_empty(&self) -> bool {
            self.bytes.is_empty()
        }
    }

    impl Read for MemoryPipe {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let length = buffer.len().min(self.bytes.len());
            buffer[..length].copy_from_slice(&self.bytes[..length]);
            self.bytes.drain(..length);
            Ok(length)
        }
    }

    impl Write for MemoryPipe {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct CertificateFixture {
        certificate_pem: String,
        private_key_pem: String,
        certificate_der: CertificateDer<'static>,
        private_key: PrivateKeyDer<'static>,
    }

    fn ca_issuer() -> (CertificateDer<'static>, String, Issuer<'static, KeyPair>) {
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("CA parameters");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca_key = KeyPair::generate().expect("CA key");
        let ca_certificate = ca_params
            .self_signed(&ca_key)
            .expect("self-signed CA certificate");
        let ca_pem = ca_certificate.pem();
        let issuer = Issuer::new(ca_params, ca_key);
        (ca_certificate.der().clone(), ca_pem, issuer)
    }

    fn issued_certificate(
        issuer: &Issuer<'static, KeyPair>,
        common_name: &str,
        extended_key_usage: ExtendedKeyUsagePurpose,
    ) -> CertificateFixture {
        let mut params = CertificateParams::new(vec![common_name.to_string()])
            .expect("leaf certificate parameters");
        params.extended_key_usages = vec![extended_key_usage.clone()];
        if extended_key_usage == ExtendedKeyUsagePurpose::ClientAuth {
            params.subject_alt_names.push(SanType::URI(
                format!("spiffe://solana-tax-protocol/{common_name}")
                    .try_into()
                    .expect("valid URI SAN"),
            ));
        }
        let key = KeyPair::generate().expect("leaf key");
        let certificate = params.signed_by(&key, issuer).expect("leaf certificate");
        CertificateFixture {
            certificate_pem: certificate.pem(),
            private_key_pem: key.serialize_pem(),
            certificate_der: certificate.der().clone(),
            private_key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
        }
    }

    fn allowed_client_uris() -> Vec<String> {
        vec![TEST_CLIENT_URI.to_string()]
    }

    fn write_tls_fixture(
        server: &CertificateFixture,
        client_ca_pem: &str,
    ) -> (TlsFiles, Vec<std::path::PathBuf>) {
        static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "solana-tax-protocol-tax-engine-mtls-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).expect("fixture directory");
        let paths = vec![
            directory.join("server.crt"),
            directory.join("server.key"),
            directory.join("client-ca.crt"),
        ];
        fs::write(&paths[0], &server.certificate_pem).expect("server certificate");
        fs::write(&paths[1], &server.private_key_pem).expect("server key");
        fs::write(&paths[2], client_ca_pem).expect("client CA");
        (
            TlsFiles {
                certificate: paths[0].clone(),
                private_key: paths[1].clone(),
                client_ca: paths[2].clone(),
            },
            paths,
        )
    }

    fn cleanup_tls_fixture(paths: Vec<std::path::PathBuf>) {
        if let Some(directory) = paths.first().and_then(|path| path.parent()) {
            fs::remove_dir_all(directory).expect("remove fixture directory");
        }
    }

    fn complete_handshake(
        server_config: Arc<rustls::ServerConfig>,
        client_config: Arc<ClientConfig>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut server = ServerConnection::new(server_config)?;
        let mut client =
            ClientConnection::new(client_config, ServerName::try_from("localhost")?.to_owned())?;
        let mut client_to_server = MemoryPipe { bytes: Vec::new() };
        let mut server_to_client = MemoryPipe { bytes: Vec::new() };
        for _ in 0..32 {
            if client.wants_write() {
                client.write_tls(&mut client_to_server)?;
            }
            if !client_to_server.is_empty() {
                server.read_tls(&mut client_to_server)?;
                server.process_new_packets()?;
            }
            if server.wants_write() {
                server.write_tls(&mut server_to_client)?;
            }
            if !server_to_client.is_empty() {
                client.read_tls(&mut server_to_client)?;
                client.process_new_packets()?;
            }
            if !client.is_handshaking() && !server.is_handshaking() {
                return Ok(());
            }
        }
        Err("TLS handshake did not converge within 32 flights".into())
    }

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
        assert!(
            Config::from_values("local", "0.0.0.0:8084", "", "64", "32", (None, None, None))
                .is_err()
        );
        assert!(
            Config::from_values(
                "local",
                "not-an-address",
                "token",
                "64",
                "32",
                (None, None, None)
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "local",
                "0.0.0.0:8084",
                "token",
                "0",
                "32",
                (None, None, None)
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "local",
                "0.0.0.0:8084",
                "token",
                "64",
                "0",
                (None, None, None)
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "staging",
                "0.0.0.0:8084",
                "local-development-only",
                "64",
                "32",
                (None, None, None),
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "staging",
                "0.0.0.0:8084",
                "token",
                "64",
                "32",
                (None, None, None)
            )
            .is_err()
        );
        assert!(
            Config::from_values(
                "staging",
                "0.0.0.0:8084",
                "token",
                "64",
                "32",
                (Some("cert.pem"), Some("key.pem"), None)
            )
            .is_err()
        );
        assert_eq!(
            TlsFiles::from_values(Some("cert.pem"), Some("key.pem"), Some("ca.pem")),
            Ok(Some(TlsFiles {
                certificate: "cert.pem".into(),
                private_key: "key.pem".into(),
                client_ca: "ca.pem".into(),
            }))
        );
    }

    #[test]
    fn non_local_identity_policy_is_required_and_bounded() {
        let tls = (Some("cert.pem"), Some("key.pem"), Some("ca.pem"));
        assert!(
            Config::from_values_with_identity(
                "staging",
                "0.0.0.0:8084",
                "token",
                "64",
                "32",
                tls,
                None,
            )
            .is_err()
        );
        assert!(
            Config::from_values_with_identity(
                "staging",
                "0.0.0.0:8084",
                "token",
                "64",
                "32",
                tls,
                Some("[\"spiffe://solana-tax-protocol/api\",\"spiffe://solana-tax-protocol/api\"]"),
            )
            .is_err()
        );
        assert!(
            Config::from_values_with_identity(
                "staging",
                "0.0.0.0:8084",
                "token",
                "64",
                "32",
                tls,
                Some("[\"https://not-spiffe.example/api\"]"),
            )
            .is_err()
        );
        let config = Config::from_values_with_identity(
            "staging",
            "0.0.0.0:8084",
            "token",
            "64",
            "32",
            tls,
            Some("[\"spiffe://solana-tax-protocol/api\"]"),
        )
        .expect("valid non-local identity policy");
        assert_eq!(
            config.allowed_client_uris,
            vec!["spiffe://solana-tax-protocol/api"]
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

    #[test]
    fn malformed_tls_material_fails_closed_before_listener_creation() {
        let path = std::env::temp_dir().join(format!(
            "solana-tax-protocol-invalid-tax-engine-cert-{}",
            std::process::id()
        ));
        std::fs::write(&path, b"not-a-certificate").expect("test fixture writes");
        let files = TlsFiles {
            certificate: path.clone(),
            private_key: path.clone(),
            client_ca: path.clone(),
        };
        assert!(super::build_mtls_config(&files, &allowed_client_uris()).is_err());
        std::fs::remove_file(path).expect("test fixture removes");
    }

    #[test]
    fn trusted_client_certificate_completes_the_mtls_handshake() {
        let (ca_der, ca_pem, issuer) = ca_issuer();
        let server = issued_certificate(&issuer, "localhost", ExtendedKeyUsagePurpose::ServerAuth);
        let client = issued_certificate(
            &issuer,
            "tax-engine-client",
            ExtendedKeyUsagePurpose::ClientAuth,
        );
        let (files, paths) = write_tls_fixture(&server, &ca_pem);
        let allowed = allowed_client_uris();
        let server_config = Arc::new(
            build_mtls_server_config(&files, &allowed).expect("valid server mTLS configuration"),
        );
        let mut roots = RootCertStore::empty();
        roots.add(ca_der).expect("server CA root");
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_auth_cert(vec![client.certificate_der], client.private_key)
                .expect("valid client mTLS configuration"),
        );
        complete_handshake(server_config, client_config).expect("trusted mTLS handshake");
        cleanup_tls_fixture(paths);
    }

    #[test]
    fn untrusted_client_certificate_is_rejected_before_http_authentication() {
        let (trusted_ca_der, trusted_ca_pem, trusted_issuer) = ca_issuer();
        let (_, _, untrusted_issuer) = ca_issuer();
        let server = issued_certificate(
            &trusted_issuer,
            "localhost",
            ExtendedKeyUsagePurpose::ServerAuth,
        );
        let client = issued_certificate(
            &untrusted_issuer,
            "untrusted-client",
            ExtendedKeyUsagePurpose::ClientAuth,
        );
        let (files, paths) = write_tls_fixture(&server, &trusted_ca_pem);
        let allowed = allowed_client_uris();
        let server_config = Arc::new(
            build_mtls_server_config(&files, &allowed).expect("valid server mTLS configuration"),
        );
        let mut roots = RootCertStore::empty();
        roots.add(trusted_ca_der).expect("server CA root");
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_auth_cert(vec![client.certificate_der], client.private_key)
                .expect("valid client mTLS configuration"),
        );
        assert!(complete_handshake(server_config, client_config).is_err());
        cleanup_tls_fixture(paths);
    }

    #[test]
    fn trusted_but_wrong_client_identity_is_rejected_during_the_handshake() {
        let (ca_der, ca_pem, issuer) = ca_issuer();
        let server = issued_certificate(&issuer, "localhost", ExtendedKeyUsagePurpose::ServerAuth);
        let client = issued_certificate(
            &issuer,
            "wrong-tax-engine-client",
            ExtendedKeyUsagePurpose::ClientAuth,
        );
        let (files, paths) = write_tls_fixture(&server, &ca_pem);
        let allowed = allowed_client_uris();
        let server_config = Arc::new(
            build_mtls_server_config(&files, &allowed).expect("valid server mTLS configuration"),
        );
        let mut roots = RootCertStore::empty();
        roots.add(ca_der).expect("server CA root");
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_auth_cert(vec![client.certificate_der], client.private_key)
                .expect("valid client mTLS configuration"),
        );
        assert!(complete_handshake(server_config, client_config).is_err());
        cleanup_tls_fixture(paths);
    }

    #[test]
    fn valid_mtls_reload_atomically_swaps_the_server_configuration() {
        let (ca_der, ca_pem, issuer) = ca_issuer();
        let server_v1 =
            issued_certificate(&issuer, "localhost", ExtendedKeyUsagePurpose::ServerAuth);
        let client = issued_certificate(
            &issuer,
            "tax-engine-client",
            ExtendedKeyUsagePurpose::ClientAuth,
        );
        let (files, paths) = write_tls_fixture(&server_v1, &ca_pem);
        let allowed = allowed_client_uris();
        let tls_config =
            build_mtls_config(&files, &allowed).expect("valid initial mTLS configuration");
        let previous = tls_config.get_inner();
        let server_v2 =
            issued_certificate(&issuer, "localhost", ExtendedKeyUsagePurpose::ServerAuth);
        fs::write(&files.certificate, &server_v2.certificate_pem).expect("replacement certificate");
        fs::write(&files.private_key, &server_v2.private_key_pem).expect("replacement key");

        reload_mtls_config(&tls_config, &files, &allowed)
            .expect("valid replacement mTLS configuration");

        let current = tls_config.get_inner();
        assert!(!Arc::ptr_eq(&previous, &current));
        let mut roots = RootCertStore::empty();
        roots.add(ca_der).expect("server CA root");
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_auth_cert(vec![client.certificate_der], client.private_key)
                .expect("valid client mTLS configuration"),
        );
        complete_handshake(current, client_config).expect("reloaded mTLS handshake");
        cleanup_tls_fixture(paths);
    }

    #[test]
    fn invalid_mtls_reload_retains_the_previous_server_configuration() {
        let (ca_der, ca_pem, issuer) = ca_issuer();
        let server = issued_certificate(&issuer, "localhost", ExtendedKeyUsagePurpose::ServerAuth);
        let client = issued_certificate(
            &issuer,
            "tax-engine-client",
            ExtendedKeyUsagePurpose::ClientAuth,
        );
        let (files, paths) = write_tls_fixture(&server, &ca_pem);
        let allowed = allowed_client_uris();
        let tls_config =
            build_mtls_config(&files, &allowed).expect("valid initial mTLS configuration");
        let previous = tls_config.get_inner();
        fs::write(&files.certificate, b"not-a-certificate")
            .expect("invalid replacement certificate");

        assert!(reload_mtls_config(&tls_config, &files, &allowed).is_err());

        let current = tls_config.get_inner();
        assert!(Arc::ptr_eq(&previous, &current));
        let mut roots = RootCertStore::empty();
        roots.add(ca_der).expect("server CA root");
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_auth_cert(vec![client.certificate_der], client.private_key)
                .expect("valid client mTLS configuration"),
        );
        complete_handshake(current, client_config)
            .expect("previous mTLS configuration remains live");
        cleanup_tls_fixture(paths);
    }
}
