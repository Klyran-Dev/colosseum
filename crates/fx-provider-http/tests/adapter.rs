use fx_provider::{AsyncFxQuoteSource, FxQuoteRequest, LegalAcceptance};
use fx_provider_http::{FxHttpEnvironment, HttpFxQuoteSource, HttpFxQuoteSourceConfig};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;

const AUTHORITY_DIGEST: &str = "0101010101010101010101010101010101010101010101010101010101010101";

fn request() -> FxQuoteRequest {
    FxQuoteRequest {
        merchant_id: [2; 32],
        facts_digest: [3; 32],
        source_currency: "USD".to_string(),
        target_currency: "EUR".to_string(),
    }
}

fn response_body(unknown_field: bool) -> String {
    let suffix = if unknown_field {
        ",\"unexpected\":true"
    } else {
        ""
    };
    format!(
        "{{\"source_currency\":\"USD\",\"target_currency\":\"EUR\",\"numerator\":\"9\",\"denominator\":\"8\",\"rounding_mode\":\"HalfUp\",\"quantum_atomic\":\"1\",\"quote_id\":\"quote-1\",\"observed_at_unix\":1800000090,\"expires_at_unix\":1800000120,\"legal_acceptance\":{{\"Approved\":{{\"authority_digest\":\"{AUTHORITY_DIGEST}\"}}}}{suffix}}}"
    )
}

async fn fixture_server(
    status: u16,
    content_type: &str,
    body: String,
) -> (String, tokio::task::JoinHandle<()>) {
    fixture_server_with_delay(status, content_type, body, Duration::ZERO).await
}

async fn fixture_server_with_delay(
    status: u16,
    content_type: &str,
    body: String,
    delay: Duration,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture");
    let address = listener.local_addr().expect("fixture address");
    let response = format!(
        "HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("fixture accept");
        let mut request_bytes = [0_u8; 4096];
        let _ = socket.read(&mut request_bytes).await;
        sleep(delay).await;
        let _ = socket.write_all(response.as_bytes()).await;
    });
    (format!("http://{address}"), handle)
}

fn config() -> HttpFxQuoteSourceConfig {
    HttpFxQuoteSourceConfig {
        environment: FxHttpEnvironment::LocalDevelopment,
        timeout: Duration::from_millis(500),
        max_response_bytes: 32 * 1024,
    }
}

#[tokio::test]
async fn accepts_strict_response_and_hashes_exact_raw_body() {
    let body = response_body(false);
    let expected_digest: [u8; 32] = Sha256::digest(body.as_bytes()).into();
    let (endpoint, server) = fixture_server(200, "application/json", body).await;
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");

    let candidate = source.quote(&request()).await.expect("valid quote");

    assert_eq!(candidate.provider, "provider-a");
    assert_eq!(candidate.numerator, 9);
    assert_eq!(candidate.denominator, 8);
    assert_eq!(
        candidate.legal_acceptance,
        LegalAcceptance::Approved {
            authority_digest: [1; 32]
        }
    );
    assert_eq!(candidate.evidence_digest, expected_digest);
    server.await.expect("fixture completed");
}

#[test]
fn rejects_non_loopback_http_and_redacts_credentials_from_debug() {
    let error = HttpFxQuoteSource::new(
        "provider-a",
        "http://example.com:80/quote",
        "secret-token",
        config(),
    )
    .expect_err("non-loopback HTTP must fail closed");
    assert!(error.to_string().contains("loopback"));
    let staging_error = HttpFxQuoteSource::new(
        "provider-a",
        "http://provider.example:80/quote",
        "secret-token",
        HttpFxQuoteSourceConfig {
            environment: FxHttpEnvironment::Staging,
            ..config()
        },
    )
    .expect_err("staging must require HTTPS");
    assert!(staging_error.to_string().contains("HTTPS"));

    let (endpoint, _) = tokio::runtime::Runtime::new()
        .expect("runtime")
        .block_on(fixture_server(
            200,
            "application/json",
            response_body(false),
        ));
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");
    assert!(!format!("{source:?}").contains("secret-token"));
}

#[tokio::test]
async fn rejects_unknown_fields_and_non_success_statuses() {
    let (endpoint, server) = fixture_server(200, "application/json", response_body(true)).await;
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");

    let body = response_body(false).replacen("\"EUR\"", "\"GBP\"", 1);
    let (endpoint, server) = fixture_server(200, "application/json", body).await;
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");

    let (endpoint, server) = fixture_server(503, "application/json", response_body(false)).await;
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");

    let (endpoint, server) = fixture_server(200, "text/plain", response_body(false)).await;
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");
}

#[tokio::test]
async fn rejects_oversized_body_and_non_canonical_decimal() {
    let (endpoint, server) = fixture_server(200, "application/json", response_body(false)).await;
    let source = HttpFxQuoteSource::new(
        "provider-a",
        &endpoint,
        "secret-token",
        HttpFxQuoteSourceConfig {
            max_response_bytes: 32,
            ..config()
        },
    )
    .expect("valid bounded source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");

    let body = response_body(false).replace("\"9\"", "\"+9\"");
    let (endpoint, server) = fixture_server(200, "application/json", body).await;
    let source = HttpFxQuoteSource::new("provider-a", &endpoint, "secret-token", config())
        .expect("valid local source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");
}

#[tokio::test]
async fn enforces_transport_deadline_without_retrying() {
    let (endpoint, server) = fixture_server_with_delay(
        200,
        "application/json",
        response_body(false),
        Duration::from_millis(100),
    )
    .await;
    let source = HttpFxQuoteSource::new(
        "provider-a",
        &endpoint,
        "secret-token",
        HttpFxQuoteSourceConfig {
            timeout: Duration::from_millis(10),
            ..config()
        },
    )
    .expect("valid local source");
    assert!(source.quote(&request()).await.is_err());
    server.await.expect("fixture completed");
}
