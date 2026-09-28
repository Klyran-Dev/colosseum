#![deny(unsafe_code)]

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;
use std::{
    env,
    net::SocketAddr,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{Notify, RwLock},
    time::sleep,
};

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:8899";
const DEFAULT_BLOCKHASH: &str = "11111111111111111111111111111111";
const DEFAULT_VAULT_ADDRESS: &str = "Sysvar1nstructions1111111111111111111111111";
const DEFAULT_VAULT_MINT: &str = "So11111111111111111111111111111111111111112";
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SETTLEMENT_PROGRAM_ID: &str = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
const ADJUSTMENT_REGISTRY_ACCOUNT_DATA_LEN: usize = 8 + (5 * 32) + 2;
const MAX_FAIL_METHODS: usize = 8;
const MAX_FAIL_METHOD_CONFIG_BYTES: usize = 256;
const MAX_TRANSACTION_BYTES: usize = 1_232;
const AMBIGUOUS_PROVIDER_ERROR: (i64, &str) = (
    -32_000,
    "transaction accepted but provider response was unavailable",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SendOutcome {
    Success,
    Ambiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedStatus {
    Unknown,
    Confirmed,
}

#[derive(Clone)]
struct FixtureState {
    blockhash: String,
    fail_methods: Arc<Vec<String>>,
    vault_address: String,
    vault_mint: Pubkey,
    vault_amount: Arc<AtomicU64>,
    vault_slot: Arc<AtomicU64>,
    vault_read_count: Arc<AtomicU64>,
    adjustment_registry: Option<AdjustmentRegistryFixture>,
    send_delay: Duration,
    send_count: Arc<AtomicU64>,
    send_outcome: Arc<RwLock<SendOutcome>>,
    observed_status: Arc<RwLock<ObservedStatus>>,
    last_signature: Arc<RwLock<Option<String>>>,
    hold_send: Arc<RwLock<bool>>,
    release_send: Arc<Notify>,
}

#[derive(Clone)]
struct AdjustmentRegistryFixture {
    address: String,
    slot: u64,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Serialize)]
struct SendCountResponse {
    send_transaction_calls: u64,
}

#[derive(Debug, Serialize)]
struct VaultReadCountResponse {
    vault_read_calls: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = env::var("RPC_FIXTURE_BIND_ADDRESS")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string())
        .parse::<SocketAddr>()?;
    let blockhash =
        env::var("RPC_FIXTURE_BLOCKHASH").unwrap_or_else(|_| DEFAULT_BLOCKHASH.to_string());
    if blockhash.is_empty() || blockhash.len() > 64 {
        return Err("RPC_FIXTURE_BLOCKHASH must be a bounded non-empty base58 value".into());
    }
    let send_delay_ms = env::var("RPC_FIXTURE_SEND_DELAY_MS")
        .unwrap_or_else(|_| "25".to_string())
        .parse::<u64>()?;
    if send_delay_ms > 2_000 {
        return Err("RPC_FIXTURE_SEND_DELAY_MS must be at most 2000ms".into());
    }
    let vault_address =
        env::var("RPC_FIXTURE_VAULT_ADDRESS").unwrap_or_else(|_| DEFAULT_VAULT_ADDRESS.to_string());
    Pubkey::from_str(&vault_address)
        .map_err(|_| "RPC_FIXTURE_VAULT_ADDRESS must be a valid public key")?;
    let vault_mint_value =
        env::var("RPC_FIXTURE_VAULT_MINT").unwrap_or_else(|_| DEFAULT_VAULT_MINT.to_string());
    let vault_mint = Pubkey::from_str(&vault_mint_value)
        .map_err(|_| "RPC_FIXTURE_VAULT_MINT must be a valid public key")?;
    let vault_amount = env::var("RPC_FIXTURE_VAULT_AMOUNT")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<u64>()?;
    let vault_slot = env::var("RPC_FIXTURE_VAULT_SLOT")
        .unwrap_or_else(|_| "42".to_string())
        .parse::<u64>()?;
    let adjustment_registry = load_adjustment_registry_fixture()?;
    let fail_method_config = env::var("RPC_FIXTURE_FAIL_METHODS").ok();
    let fail_methods = parse_fail_methods(fail_method_config.as_deref())
        .map_err(|error| format!("RPC_FIXTURE_FAIL_METHODS is invalid: {error}"))?;
    let state = FixtureState {
        blockhash,
        fail_methods: Arc::new(fail_methods),
        vault_address,
        vault_mint,
        vault_amount: Arc::new(AtomicU64::new(vault_amount)),
        vault_slot: Arc::new(AtomicU64::new(vault_slot)),
        vault_read_count: Arc::new(AtomicU64::new(0)),
        adjustment_registry,
        send_delay: Duration::from_millis(send_delay_ms),
        send_count: Arc::new(AtomicU64::new(0)),
        send_outcome: Arc::new(RwLock::new(SendOutcome::Success)),
        observed_status: Arc::new(RwLock::new(ObservedStatus::Unknown)),
        last_signature: Arc::new(RwLock::new(None)),
        hold_send: Arc::new(RwLock::new(false)),
        release_send: Arc::new(Notify::new()),
    };
    let app = Router::new()
        .route("/", axum::routing::post(json_rpc))
        .route("/healthz", get(healthz))
        .route("/admin/send-count", get(send_count))
        .route("/admin/ambiguous", post(enable_ambiguous_outcome))
        .route("/admin/hold", post(hold_send))
        .route("/admin/release", post(release_send))
        .route("/admin/vault-amount", post(set_vault_amount))
        .route("/admin/vault-read-count", get(vault_read_count))
        .with_state(state);
    let listener = TcpListener::bind(bind_address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn send_count(State(state): State<FixtureState>) -> Json<SendCountResponse> {
    Json(SendCountResponse {
        send_transaction_calls: state.send_count.load(Ordering::Acquire),
    })
}

async fn vault_read_count(State(state): State<FixtureState>) -> Json<VaultReadCountResponse> {
    Json(VaultReadCountResponse {
        vault_read_calls: state.vault_read_count.load(Ordering::Acquire),
    })
}

/// Enables one local-only accepted-but-ambiguous outcome. This endpoint is
/// intentionally exposed only by the synthetic fixture Compose profile; it is
/// not part of the production API or coordinator contract.
async fn enable_ambiguous_outcome(State(state): State<FixtureState>) -> StatusCode {
    *state.send_outcome.write().await = SendOutcome::Ambiguous;
    *state.observed_status.write().await = ObservedStatus::Confirmed;
    StatusCode::NO_CONTENT
}

/// Resets the fixture into a response-hold mode for the process-crash drill.
async fn hold_send(State(state): State<FixtureState>) -> StatusCode {
    *state.send_outcome.write().await = SendOutcome::Success;
    *state.observed_status.write().await = ObservedStatus::Unknown;
    *state.last_signature.write().await = None;
    *state.hold_send.write().await = true;
    StatusCode::NO_CONTENT
}

/// Releases a held response and makes the accepted signature observable.
async fn release_send(State(state): State<FixtureState>) -> StatusCode {
    *state.observed_status.write().await = ObservedStatus::Confirmed;
    *state.hold_send.write().await = false;
    state.release_send.notify_waiters();
    StatusCode::NO_CONTENT
}

#[derive(Debug, Deserialize)]
struct VaultAmountUpdate {
    amount_atomic: u64,
    #[serde(default)]
    slot: Option<u64>,
}

/// Fixture-only mutation used to produce deterministic mismatch evidence. It
/// is not an RPC or production API and is reachable only inside the isolated
/// settlement-submit-drill Compose profile.
async fn set_vault_amount(
    State(state): State<FixtureState>,
    Json(update): Json<VaultAmountUpdate>,
) -> StatusCode {
    state
        .vault_amount
        .store(update.amount_atomic, Ordering::Release);
    if let Some(slot) = update.slot {
        state.vault_slot.store(slot, Ordering::Release);
    }
    StatusCode::NO_CONTENT
}

async fn json_rpc(
    State(state): State<FixtureState>,
    Json(request): Json<RpcRequest>,
) -> Json<RpcResponse> {
    let id = request.id.clone();
    let response = if request.jsonrpc != "2.0" || request.method.is_empty() {
        error_response(id, -32600, "invalid JSON-RPC request")
    } else {
        match handle_method(&state, &request.method, &request.params).await {
            Ok(result) => RpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(result),
                error: None,
            },
            Err((code, message)) => error_response(id, code, &message),
        }
    };
    Json(response)
}

async fn handle_method(
    state: &FixtureState,
    method: &str,
    params: &Value,
) -> Result<Value, (i64, String)> {
    if method == "getMultipleAccounts" {
        state.vault_read_count.fetch_add(1, Ordering::AcqRel);
    }
    if state
        .fail_methods
        .iter()
        .any(|configured| configured == method)
    {
        return Err((
            -32_001,
            format!("fixture configured to fail method: {method}"),
        ));
    }
    match method {
        "getLatestBlockhash" => Ok(json!({
            "context": {"slot": 1},
            "value": {
                "blockhash": state.blockhash,
                "lastValidBlockHeight": 1_000_000u64
            }
        })),
        "getSlot" => Ok(json!(42u64)),
        // The reconciliation drill has no synthetic settlement blocks. Returning
        // an empty finalized page keeps the indexer's replay path healthy while
        // leaving vault-account reads as the only failover assertion.
        "getBlocks" => Ok(json!([])),
        "isBlockhashValid" => Ok(json!({"context": {"slot": 1}, "value": true})),
        "getMultipleAccounts" => get_multiple_accounts(state, params),
        "getAccountInfo" => get_account_info(state, params),
        "sendTransaction" => send_transaction(state, params).await,
        "getSignatureStatuses" => signature_statuses(state, params).await,
        _ => Err((-32601, format!("unsupported fixture RPC method: {method}"))),
    }
}

fn load_adjustment_registry_fixture()
-> Result<Option<AdjustmentRegistryFixture>, Box<dyn std::error::Error>> {
    let names = [
        "RPC_FIXTURE_REGISTRY_MERCHANT",
        "RPC_FIXTURE_REGISTRY_MINT",
        "RPC_FIXTURE_REGISTRY_PROPOSER",
        "RPC_FIXTURE_REGISTRY_APPROVER",
    ];
    let values = names
        .iter()
        .map(|name| env::var(name).ok())
        .collect::<Vec<_>>();
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    if values.iter().any(Option::is_none) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "all RPC_FIXTURE_REGISTRY_* public keys are required together",
        )
        .into());
    }
    let parse_key = |index: usize| -> Result<Pubkey, Box<dyn std::error::Error>> {
        let value = values[index].as_deref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "registry key is missing")
        })?;
        Pubkey::from_str(value).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("registry key {value} is not a valid Solana public key"),
            )
            .into()
        })
    };
    let merchant = parse_key(0)?;
    let mint = parse_key(1)?;
    let proposer = parse_key(2)?;
    let approver = parse_key(3)?;
    if proposer == approver {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "registry proposer and approver must be distinct",
        )
        .into());
    }
    let program = Pubkey::from_str(SETTLEMENT_PROGRAM_ID)?;
    let (address, bump) = Pubkey::find_program_address(
        &[b"adjustment-registry", merchant.as_ref(), mint.as_ref()],
        &program,
    );
    let mut discriminator = Sha256::new();
    discriminator.update(b"account:SettlementAdjustmentRegistry");
    let discriminator = discriminator.finalize();
    let mut data = Vec::with_capacity(ADJUSTMENT_REGISTRY_ACCOUNT_DATA_LEN);
    data.extend_from_slice(&discriminator[..8]);
    data.extend_from_slice(merchant.as_ref());
    data.extend_from_slice(mint.as_ref());
    data.extend_from_slice(&[9; 32]);
    data.extend_from_slice(proposer.as_ref());
    data.extend_from_slice(approver.as_ref());
    data.push(1);
    data.push(bump);
    debug_assert_eq!(data.len(), ADJUSTMENT_REGISTRY_ACCOUNT_DATA_LEN);
    let slot = env::var("RPC_FIXTURE_REGISTRY_SLOT")
        .unwrap_or_else(|_| "777".to_string())
        .parse::<u64>()?;
    Ok(Some(AdjustmentRegistryFixture {
        address: address.to_string(),
        slot,
        data,
    }))
}

fn parse_fail_methods(value: Option<&str>) -> Result<Vec<String>, &'static str> {
    let raw = value.unwrap_or("").trim();
    if raw.len() > MAX_FAIL_METHOD_CONFIG_BYTES {
        return Err("method configuration exceeds the byte bound");
    }
    let mut methods = Vec::new();
    for candidate in raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if candidate.len() > 64
            || !candidate
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err("method names must be bounded ASCII identifiers");
        }
        if methods.iter().any(|method| method == candidate) {
            return Err("method names must be unique");
        }
        methods.push(candidate.to_string());
        if methods.len() > MAX_FAIL_METHODS {
            return Err("too many failure methods");
        }
    }
    Ok(methods)
}

fn get_multiple_accounts(state: &FixtureState, params: &Value) -> Result<Value, (i64, String)> {
    let values = params.as_array().ok_or((
        -32602,
        "getMultipleAccounts requires an array of parameters".to_string(),
    ))?;
    let addresses = values
        .first()
        .and_then(Value::as_array)
        .ok_or((-32602, "account addresses must be an array".to_string()))?;
    if addresses.len() != 1 {
        return Err((
            -32602,
            "fixture accepts exactly one bounded account address".to_string(),
        ));
    }
    let address = addresses[0]
        .as_str()
        .ok_or((-32602, "account address must be a string".to_string()))?;
    if address != state.vault_address {
        return Err((
            -32602,
            "fixture account address is not the configured vault".to_string(),
        ));
    }
    let options = values
        .get(1)
        .and_then(Value::as_object)
        .ok_or((-32602, "account options are required".to_string()))?;
    if options.get("commitment").and_then(Value::as_str) != Some("finalized")
        || options.get("encoding").and_then(Value::as_str) != Some("base64")
    {
        return Err((
            -32602,
            "fixture requires finalized base64 account reads".to_string(),
        ));
    }
    let data = BASE64.encode(synthetic_token_account_data(
        state.vault_mint,
        state.vault_amount.load(Ordering::Acquire),
    ));
    Ok(json!({
        "context": {"slot": state.vault_slot.load(Ordering::Acquire)},
        "value": [{
            "data": [data, "base64"],
            "executable": false,
            "lamports": 2_039_280u64,
            "owner": SPL_TOKEN_PROGRAM_ID,
            "rentEpoch": u64::MAX
        }]
    }))
}

fn get_account_info(state: &FixtureState, params: &Value) -> Result<Value, (i64, String)> {
    let values = params.as_array().ok_or((
        -32602,
        "getAccountInfo requires an array of parameters".to_string(),
    ))?;
    let address = values
        .first()
        .and_then(Value::as_str)
        .ok_or((-32602, "account address must be a string".to_string()))?;
    let options = values
        .get(1)
        .and_then(Value::as_object)
        .ok_or((-32602, "account options are required".to_string()))?;
    if options.get("commitment").and_then(Value::as_str) != Some("finalized")
        || options.get("encoding").and_then(Value::as_str) != Some("base64")
    {
        return Err((
            -32602,
            "fixture requires finalized base64 account reads".to_string(),
        ));
    }
    let registry = state.adjustment_registry.as_ref().ok_or((
        -32602,
        "adjustment registry fixture is not configured".to_string(),
    ))?;
    if address != registry.address {
        return Err((
            -32602,
            "fixture account address is not the configured adjustment registry".to_string(),
        ));
    }
    Ok(json!({
        "context": {"slot": registry.slot},
        "value": {
            "data": [BASE64.encode(&registry.data), "base64"],
            "executable": false,
            "lamports": 1_000_000u64,
            "owner": SETTLEMENT_PROGRAM_ID,
            "rentEpoch": u64::MAX
        }
    }))
}

fn synthetic_token_account_data(mint: Pubkey, amount_atomic: u64) -> Vec<u8> {
    let mut data = vec![0u8; 165];
    data[0..32].copy_from_slice(mint.as_ref());
    data[64..72].copy_from_slice(&amount_atomic.to_le_bytes());
    data
}

async fn send_transaction(state: &FixtureState, params: &Value) -> Result<Value, (i64, String)> {
    let encoded = params
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .ok_or((
            -32602,
            "sendTransaction requires a base64 transaction".to_string(),
        ))?;
    let bytes = BASE64.decode(encoded).map_err(|_| {
        (
            -32602,
            "sendTransaction transaction is not base64".to_string(),
        )
    })?;
    if bytes.is_empty() || bytes.len() > MAX_TRANSACTION_BYTES {
        return Err((
            -32602,
            "sendTransaction transaction exceeds the bounded packet size".to_string(),
        ));
    }
    let transaction = wincode::deserialize_exact::<VersionedTransaction>(&bytes)
        .map_err(|error| (-32602, format!("invalid versioned transaction: {error}")))?;
    transaction.verify_and_hash_message().map_err(|error| {
        (
            -32602,
            format!("transaction signature verification failed: {error}"),
        )
    })?;
    let signature = transaction
        .signatures
        .first()
        .ok_or((-32602, "transaction has no payer signature".to_string()))?;
    let signature = signature.to_string();
    *state.last_signature.write().await = Some(signature.clone());
    state.send_count.fetch_add(1, Ordering::AcqRel);
    sleep(state.send_delay).await;
    let released = state.release_send.notified();
    if *state.hold_send.read().await {
        released.await;
    }
    match *state.send_outcome.read().await {
        SendOutcome::Success => Ok(Value::String(signature)),
        SendOutcome::Ambiguous => Err((
            AMBIGUOUS_PROVIDER_ERROR.0,
            AMBIGUOUS_PROVIDER_ERROR.1.to_string(),
        )),
    }
}

async fn signature_statuses(state: &FixtureState, params: &Value) -> Result<Value, (i64, String)> {
    let requested_signature = params
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str);
    let accepted_signature = state.last_signature.read().await.clone();
    let observed_status = *state.observed_status.read().await;
    let status = signature_status_value(
        accepted_signature.as_deref(),
        requested_signature,
        observed_status,
    );
    Ok(json!({"context": {"slot": 42}, "value": [status]}))
}

fn signature_status_value(
    accepted_signature: Option<&str>,
    requested_signature: Option<&str>,
    observed_status: ObservedStatus,
) -> Value {
    if accepted_signature != requested_signature || observed_status == ObservedStatus::Unknown {
        Value::Null
    } else {
        json!({
            "slot": 42,
            "confirmations": null,
            "confirmationStatus": "confirmed",
            "err": null
        })
    }
}

fn error_response(id: Value, code: i64, message: &str) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError {
            code,
            message: message.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_failure_methods_are_trimmed_and_bounded() {
        assert_eq!(
            parse_fail_methods(Some(" getMultipleAccounts, sendTransaction ")).unwrap(),
            vec!["getMultipleAccounts", "sendTransaction"]
        );
        assert!(parse_fail_methods(Some(" ")).unwrap().is_empty());
        assert!(parse_fail_methods(Some(&"x".repeat(257))).is_err());
    }

    #[test]
    fn synthetic_token_account_uses_canonical_spl_offsets() {
        let mint = Pubkey::new_from_array([7; 32]);
        let data = synthetic_token_account_data(mint, 4_294_967_297);

        assert_eq!(&data[0..32], mint.as_ref());
        assert_eq!(
            u64::from_le_bytes(data[64..72].try_into().unwrap()),
            4_294_967_297
        );
        assert_eq!(data.len(), 165);
    }

    #[test]
    fn unknown_status_does_not_attest_to_an_unrelated_signature() {
        assert_eq!(
            signature_status_value(
                Some("accepted-signature"),
                Some("different-signature"),
                ObservedStatus::Confirmed,
            ),
            Value::Null
        );
    }

    #[test]
    fn confirmed_status_is_returned_only_for_the_exact_accepted_signature() {
        let status = signature_status_value(
            Some("accepted-signature"),
            Some("accepted-signature"),
            ObservedStatus::Confirmed,
        );
        assert_eq!(status["confirmationStatus"], "confirmed");
        assert_eq!(status["slot"], 42);
    }

    #[tokio::test]
    async fn finalized_block_page_is_empty_for_reconciliation_fixture() {
        let state = FixtureState {
            blockhash: DEFAULT_BLOCKHASH.to_string(),
            fail_methods: Arc::new(Vec::new()),
            vault_address: DEFAULT_VAULT_ADDRESS.to_string(),
            vault_mint: Pubkey::from_str(DEFAULT_VAULT_MINT).unwrap(),
            vault_amount: Arc::new(AtomicU64::new(1000)),
            vault_slot: Arc::new(AtomicU64::new(42)),
            vault_read_count: Arc::new(AtomicU64::new(0)),
            adjustment_registry: None,
            send_delay: Duration::ZERO,
            send_count: Arc::new(AtomicU64::new(0)),
            send_outcome: Arc::new(RwLock::new(SendOutcome::Success)),
            observed_status: Arc::new(RwLock::new(ObservedStatus::Unknown)),
            last_signature: Arc::new(RwLock::new(None)),
            hold_send: Arc::new(RwLock::new(false)),
            release_send: Arc::new(Notify::new()),
        };

        assert_eq!(
            handle_method(&state, "getBlocks", &json!([]))
                .await
                .unwrap(),
            json!([])
        );
    }
}
