use base64::{Engine, engine::general_purpose::STANDARD};
use fx_provider::AsyncFxEvidenceSigner;
use fx_signer_http::{FxSignerHttpEnvironment, HttpFxEvidenceSigner, HttpFxEvidenceSignerConfig};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::sleep;

const KEY_ID: [u8; 32] = [1; 32];
const PUBLIC_KEY: [u8; 32] = [2; 32];

fn config() -> HttpFxEvidenceSignerConfig {
    HttpFxEvidenceSignerConfig {
        environment: FxSignerHttpEnvironment::LocalDevelopment,
        timeout: Duration::from_millis(500),
        max_response_bytes: 4096,
    }
}

async fn fixture_server(
    status: u16,
    body: String,
) -> (String, Arc<Mutex<Vec<u8>>>, tokio::task::JoinHandle<()>) {
    fixture_server_with_delay(status, body, Duration::ZERO).await
}

async fn fixture_server_with_delay(
    status: u16,
    body: String,
    delay: Duration,
) -> (String, Arc<Mutex<Vec<u8>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture");
    let address = listener.local_addr().expect("fixture address");
    let request_capture = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&request_capture);
    let response = format!(
        "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("fixture accept");
        let mut request_bytes = [0_u8; 16384];
        let length = socket
            .read(&mut request_bytes)
            .await
            .expect("fixture request");
        captured
            .lock()
            .await
            .extend_from_slice(&request_bytes[..length]);
        sleep(delay).await;
        let _ = socket.write_all(response.as_bytes()).await;
    });
    (format!("http://{address}"), request_capture, handle)
}

fn response_body(signature: &str) -> String {
    format!(r#"{{"signature":"{signature}"}}"#)
}

#[tokio::test]
async fn sends_exact_payload_and_returns_strict_signature_bytes() {
    let signature = STANDARD.encode([9_u8; 64]);
    let (endpoint, captured, server) = fixture_server(200, response_body(&signature)).await;
    let signer = HttpFxEvidenceSigner::new(KEY_ID, PUBLIC_KEY, &endpoint, "secret-token", config())
        .expect("valid local signer");
    let payload = b"canonical-payload-v1";

    let returned = signer.sign(payload).await.expect("signature response");

    assert_eq!(returned, vec![9_u8; 64]);
    let request = String::from_utf8(captured.lock().await.clone()).expect("request text");
    assert!(request.contains("0101010101010101010101010101010101010101010101010101010101010101"));
    assert!(request.contains(&STANDARD.encode(payload)));
    server.await.expect("fixture completed");
}

#[test]
fn rejects_remote_http_and_redacts_bearer_credentials() {
    let error = HttpFxEvidenceSigner::new(
        KEY_ID,
        PUBLIC_KEY,
        "http://signer.example:80/sign",
        "secret-token",
        HttpFxEvidenceSignerConfig {
            environment: FxSignerHttpEnvironment::Production,
            ..config()
        },
    )
    .expect_err("production must require HTTPS");
    assert!(error.to_string().contains("HTTPS"));

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let (endpoint, _, _) = runtime.block_on(fixture_server(
        200,
        response_body(&STANDARD.encode([9_u8; 64])),
    ));
    let signer = HttpFxEvidenceSigner::new(KEY_ID, PUBLIC_KEY, &endpoint, "secret-token", config())
        .expect("valid local signer");
    assert!(!format!("{signer:?}").contains("secret-token"));
}

#[tokio::test]
async fn rejects_unknown_fields_bad_status_and_wrong_signature_length() {
    let (endpoint, _, server) = fixture_server(
        200,
        format!(
            r#"{{"signature":"{}","unexpected":true}}"#,
            STANDARD.encode([9_u8; 64])
        ),
    )
    .await;
    let signer = HttpFxEvidenceSigner::new(KEY_ID, PUBLIC_KEY, &endpoint, "secret-token", config())
        .expect("valid local signer");
    assert!(signer.sign(b"payload").await.is_err());
    server.await.expect("fixture completed");

    let (endpoint, _, server) =
        fixture_server(503, response_body(&STANDARD.encode([9_u8; 64]))).await;
    let signer = HttpFxEvidenceSigner::new(KEY_ID, PUBLIC_KEY, &endpoint, "secret-token", config())
        .expect("valid local signer");
    assert!(signer.sign(b"payload").await.is_err());
    server.await.expect("fixture completed");

    let (endpoint, _, server) =
        fixture_server(200, response_body(&STANDARD.encode([9_u8; 32]))).await;
    let signer = HttpFxEvidenceSigner::new(KEY_ID, PUBLIC_KEY, &endpoint, "secret-token", config())
        .expect("valid local signer");
    assert!(signer.sign(b"payload").await.is_err());
    server.await.expect("fixture completed");

    let (endpoint, _, server) =
        fixture_server(200, response_body(&STANDARD.encode([9_u8; 64]))).await;
    let signer = HttpFxEvidenceSigner::new(
        KEY_ID,
        PUBLIC_KEY,
        &endpoint,
        "secret-token",
        HttpFxEvidenceSignerConfig {
            max_response_bytes: 32,
            ..config()
        },
    )
    .expect("valid bounded signer");
    assert!(signer.sign(b"payload").await.is_err());
    server.await.expect("fixture completed");
}

#[tokio::test]
async fn enforces_timeout_without_retrying() {
    let (endpoint, _, server) = fixture_server_with_delay(
        200,
        response_body(&STANDARD.encode([9_u8; 64])),
        Duration::from_millis(100),
    )
    .await;
    let signer = HttpFxEvidenceSigner::new(
        KEY_ID,
        PUBLIC_KEY,
        &endpoint,
        "secret-token",
        HttpFxEvidenceSignerConfig {
            timeout: Duration::from_millis(10),
            ..config()
        },
    )
    .expect("valid local signer");
    assert!(signer.sign(b"payload").await.is_err());
    server.await.expect("fixture completed");
}
