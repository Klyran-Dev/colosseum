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
const REFUND_CLAIM_ACCOUNT_DATA_LEN: usize = 410;
const ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR: [u8; 8] = [112, 130, 243, 73, 61, 70, 35, 70];
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
    Finalized,
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
    refund_funding: Option<RefundFundingFixture>,
    refund_proof: Option<RefundProofFixture>,
    refund_funding_read_delay: Duration,
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

#[derive(Clone)]
struct RefundFundingFixture {
    merchant_account: String,
    mint: Pubkey,
    mint_address: String,
    tax_vault: String,
    vault_authority: String,
    refund_funder: String,
    net_source: String,
    slot: u64,
    mint_decimals: u8,
}

#[derive(Clone)]
struct RefundProofFixture {
    claim: Pubkey,
    adjustment: Pubkey,
    original_receipt: Pubkey,
    merchant: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    refund_funder: Pubkey,
    tax_vault: Pubkey,
    refund_source: Pubkey,
    payer_destination: Pubkey,
    approver: Pubkey,
    signature: String,
    reference_digest: [u8; 32],
    nonce: u64,
    gross_atomic: u64,
    tax_atomic: u64,
    net_atomic: u64,
    slot: u64,
    tax_vault_before: u64,
    tax_vault_after: u64,
    refund_source_before: u64,
    refund_source_after: u64,
    payer_destination_before: u64,
    payer_destination_after: u64,
}

impl RefundProofFixture {
    fn vault_authority(&self) -> Pubkey {
        let program = Pubkey::from_str(SETTLEMENT_PROGRAM_ID)
            .expect("the checked-in settlement program ID must be valid");
        Pubkey::find_program_address(
            &[
                b"vault-authority",
                self.merchant.as_ref(),
                self.mint.as_ref(),
            ],
            &program,
        )
        .0
    }
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
    let configured_status =
        env::var("RPC_FIXTURE_SIGNATURE_STATUS").unwrap_or_else(|_| "confirmed".to_string());
    let initial_observed_status = match configured_status.as_str() {
        "unknown" => ObservedStatus::Unknown,
        "confirmed" => ObservedStatus::Confirmed,
        "finalized" => ObservedStatus::Finalized,
        _ => {
            return Err(
                "RPC_FIXTURE_SIGNATURE_STATUS must be unknown, confirmed, or finalized".into(),
            );
        }
    };
    let refund_funding_read_delay_ms = env::var("RPC_FIXTURE_REFUND_FUNDING_READ_DELAY_MS")
        .unwrap_or_else(|_| "0".to_string())
        .parse::<u64>()?;
    if refund_funding_read_delay_ms > 2_000 {
        return Err("RPC_FIXTURE_REFUND_FUNDING_READ_DELAY_MS must be at most 2000ms".into());
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
    let refund_funding = load_refund_funding_fixture()?;
    let refund_proof = load_refund_proof_fixture()?;
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
        refund_funding,
        refund_proof,
        refund_funding_read_delay: Duration::from_millis(refund_funding_read_delay_ms),
        send_delay: Duration::from_millis(send_delay_ms),
        send_count: Arc::new(AtomicU64::new(0)),
        send_outcome: Arc::new(RwLock::new(SendOutcome::Success)),
        observed_status: Arc::new(RwLock::new(initial_observed_status)),
        last_signature: Arc::new(RwLock::new(None)),
        hold_send: Arc::new(RwLock::new(false)),
        release_send: Arc::new(Notify::new()),
    };
    let app = Router::new()
        .route("/", axum::routing::post(json_rpc))
        .route("/healthz", get(healthz))
        .route("/admin/send-count", get(send_count))
        .route("/admin/ambiguous", post(enable_ambiguous_outcome))
        .route("/admin/finalized", post(mark_finalized))
        .route("/admin/hold", post(hold_send))
        .route("/admin/release", post(release_send))
        .route("/admin/vault-amount", post(set_vault_amount))
        .route("/admin/vault-read-count", get(vault_read_count))
        .route("/admin/refund-funding-read-count", get(vault_read_count))
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

/// Marks the last accepted synthetic signature finalized. This endpoint is
/// intentionally exposed only by the local fixture profile; production code
/// obtains finality exclusively from the provider coordinator.
async fn mark_finalized(State(state): State<FixtureState>) -> StatusCode {
    *state.observed_status.write().await = ObservedStatus::Finalized;
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
        if state.refund_funding.is_some() && !state.refund_funding_read_delay.is_zero() {
            sleep(state.refund_funding_read_delay).await;
        }
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
        "getTransaction" => get_refund_proof_transaction(state, params),
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
        .map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
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

fn load_refund_funding_fixture() -> Result<Option<RefundFundingFixture>, Box<dyn std::error::Error>>
{
    let names = [
        "RPC_FIXTURE_REFUND_FUNDING_MERCHANT",
        "RPC_FIXTURE_REFUND_FUNDING_MINT",
        "RPC_FIXTURE_REFUND_FUNDING_TAX_VAULT",
        "RPC_FIXTURE_REFUND_FUNDING_VAULT_AUTHORITY",
        "RPC_FIXTURE_REFUND_FUNDING_REFUND_FUNDER",
        "RPC_FIXTURE_REFUND_FUNDING_NET_SOURCE",
    ];
    let values = names
        .iter()
        .map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
        .collect::<Vec<_>>();
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    if values.iter().any(Option::is_none) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "all RPC_FIXTURE_REFUND_FUNDING_* public keys are required together",
        )
        .into());
    }
    let value = |index: usize| -> Result<String, Box<dyn std::error::Error>> {
        values.get(index).and_then(Clone::clone).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "refund funding fixture environment value is missing",
            )
            .into()
        })
    };
    let mint_address = value(1)?;
    let mint = Pubkey::from_str(&mint_address)
        .map_err(|_| "RPC_FIXTURE_REFUND_FUNDING_MINT must be a valid public key")?;
    let slot = env::var("RPC_FIXTURE_REFUND_FUNDING_SLOT")
        .unwrap_or_else(|_| "888".to_string())
        .parse::<u64>()?;
    if slot == 0 {
        return Err("RPC_FIXTURE_REFUND_FUNDING_SLOT must be positive".into());
    }
    let mint_decimals = env::var("RPC_FIXTURE_REFUND_FUNDING_MINT_DECIMALS")
        .unwrap_or_else(|_| "6".to_string())
        .parse::<u8>()?;
    Ok(Some(RefundFundingFixture {
        merchant_account: value(0)?,
        mint,
        mint_address,
        tax_vault: value(2)?,
        vault_authority: value(3)?,
        refund_funder: value(4)?,
        net_source: value(5)?,
        slot,
        mint_decimals,
    }))
}

fn load_refund_proof_fixture() -> Result<Option<RefundProofFixture>, Box<dyn std::error::Error>> {
    let names = [
        "RPC_FIXTURE_REFUND_PROOF_CLAIM",
        "RPC_FIXTURE_REFUND_PROOF_ADJUSTMENT",
        "RPC_FIXTURE_REFUND_PROOF_ORIGINAL_RECEIPT",
        "RPC_FIXTURE_REFUND_PROOF_MERCHANT",
        "RPC_FIXTURE_REFUND_PROOF_MINT",
        "RPC_FIXTURE_REFUND_PROOF_PAYER",
        "RPC_FIXTURE_REFUND_PROOF_REFUND_FUNDER",
        "RPC_FIXTURE_REFUND_PROOF_TAX_VAULT",
        "RPC_FIXTURE_REFUND_PROOF_REFUND_SOURCE",
        "RPC_FIXTURE_REFUND_PROOF_PAYER_DESTINATION",
        "RPC_FIXTURE_REFUND_PROOF_APPROVER",
        "RPC_FIXTURE_REFUND_PROOF_SIGNATURE",
        "RPC_FIXTURE_REFUND_PROOF_REFERENCE_DIGEST_HEX",
    ];
    let values = names
        .iter()
        .map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
        .collect::<Vec<_>>();
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    if values.iter().any(Option::is_none) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "all RPC_FIXTURE_REFUND_PROOF_* identity and digest values are required together",
        )
        .into());
    }
    let required = |index: usize| -> Result<&str, Box<dyn std::error::Error>> {
        values[index].as_deref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "refund proof fixture value is missing",
            )
            .into()
        })
    };
    let parse_key = |index: usize| -> Result<Pubkey, Box<dyn std::error::Error>> {
        let value = required(index)?;
        Pubkey::from_str(value).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("refund proof public key is invalid: {value}"),
            )
            .into()
        })
    };
    let parse_u64 = |name: &str, default: u64| -> Result<u64, Box<dyn std::error::Error>> {
        env::var(name)
            .unwrap_or_else(|_| default.to_string())
            .parse::<u64>()
            .map_err(|_| format!("{name} must be a canonical u64").into())
    };
    let claim = parse_key(0)?;
    let adjustment = parse_key(1)?;
    let original_receipt = parse_key(2)?;
    let merchant = parse_key(3)?;
    let mint = parse_key(4)?;
    let payer = parse_key(5)?;
    let refund_funder = parse_key(6)?;
    let tax_vault = parse_key(7)?;
    let refund_source = parse_key(8)?;
    let payer_destination = parse_key(9)?;
    let approver = parse_key(10)?;
    let signature = required(11)?.to_string();
    if signature.len() > 128 {
        return Err("RPC_FIXTURE_REFUND_PROOF_SIGNATURE is too long".into());
    }
    let reference_digest = parse_hex_digest(
        required(12)?,
        "RPC_FIXTURE_REFUND_PROOF_REFERENCE_DIGEST_HEX",
    )?;
    let nonce = parse_u64("RPC_FIXTURE_REFUND_PROOF_NONCE", 11)?;
    let gross_atomic = parse_u64("RPC_FIXTURE_REFUND_PROOF_GROSS", 100)?;
    let tax_atomic = parse_u64("RPC_FIXTURE_REFUND_PROOF_TAX", 25)?;
    let net_atomic = parse_u64("RPC_FIXTURE_REFUND_PROOF_NET", 75)?;
    let slot = parse_u64("RPC_FIXTURE_REFUND_PROOF_SLOT", 900)?;
    let tax_vault_before = parse_u64("RPC_FIXTURE_REFUND_PROOF_TAX_VAULT_BEFORE", 1_000)?;
    let tax_vault_after = parse_u64("RPC_FIXTURE_REFUND_PROOF_TAX_VAULT_AFTER", 975)?;
    let refund_source_before = parse_u64("RPC_FIXTURE_REFUND_PROOF_SOURCE_BEFORE", 800)?;
    let refund_source_after = parse_u64("RPC_FIXTURE_REFUND_PROOF_SOURCE_AFTER", 725)?;
    let payer_destination_before = parse_u64("RPC_FIXTURE_REFUND_PROOF_DESTINATION_BEFORE", 300)?;
    let payer_destination_after = parse_u64("RPC_FIXTURE_REFUND_PROOF_DESTINATION_AFTER", 400)?;
    if nonce == 0
        || slot == 0
        || gross_atomic == 0
        || tax_atomic.checked_add(net_atomic) != Some(gross_atomic)
        || tax_vault_before.checked_sub(tax_atomic) != Some(tax_vault_after)
        || refund_source_before.checked_sub(net_atomic) != Some(refund_source_after)
        || payer_destination_before.checked_add(gross_atomic) != Some(payer_destination_after)
        || claim == adjustment
        || tax_vault == refund_source
        || tax_vault == payer_destination
        || refund_source == payer_destination
    {
        return Err("refund proof fixture identity or conserving balances are invalid".into());
    }
    let program = Pubkey::from_str(SETTLEMENT_PROGRAM_ID)?;
    let derived_claim = Pubkey::find_program_address(
        &[b"refund-claim", adjustment.as_ref(), &nonce.to_le_bytes()],
        &program,
    )
    .0;
    if claim != derived_claim {
        return Err("RPC_FIXTURE_REFUND_PROOF_CLAIM is not the derived refund claim PDA".into());
    }
    Ok(Some(RefundProofFixture {
        claim,
        adjustment,
        original_receipt,
        merchant,
        mint,
        payer,
        refund_funder,
        tax_vault,
        refund_source,
        payer_destination,
        approver,
        signature,
        reference_digest,
        nonce,
        gross_atomic,
        tax_atomic,
        net_atomic,
        slot,
        tax_vault_before,
        tax_vault_after,
        refund_source_before,
        refund_source_after,
        payer_destination_before,
        payer_destination_after,
    }))
}

fn parse_hex_digest(value: &str, name: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{name} must contain exactly 32 bytes of hex").into());
    }
    let mut digest = [0u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = (hex_nibble(chunk[0])? << 4) | hex_nibble(chunk[1])?;
    }
    if digest == [0; 32] {
        return Err(format!("{name} must not be zero").into());
    }
    Ok(digest)
}

fn hex_nibble(value: u8) -> Result<u8, Box<dyn std::error::Error>> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("invalid hexadecimal value".into()),
    }
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
    if let Some(fixture) = &state.refund_proof {
        if addresses.len() == 1 {
            return get_refund_proof_claim_account(fixture, addresses);
        }
    }
    if let Some(fixture) = &state.refund_funding {
        return get_refund_funding_accounts(fixture, addresses, values);
    }
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

fn get_refund_proof_claim_account(
    fixture: &RefundProofFixture,
    addresses: &[Value],
) -> Result<Value, (i64, String)> {
    if addresses.first().and_then(Value::as_str) != Some(fixture.claim.to_string().as_str()) {
        return Err((
            -32602,
            "fixture refund-proof claim address does not match".to_string(),
        ));
    }
    let mut data = vec![0u8; REFUND_CLAIM_ACCOUNT_DATA_LEN];
    let mut discriminator = Sha256::new();
    discriminator.update(b"account:AdjustmentRefundClaim");
    data[..8].copy_from_slice(&discriminator.finalize()[..8]);
    for (offset, value) in [
        (8, fixture.adjustment),
        (40, fixture.original_receipt),
        (72, fixture.merchant),
        (104, fixture.mint),
        (136, fixture.payer),
        (168, fixture.refund_funder),
        (200, fixture.tax_vault),
        (232, fixture.refund_source),
        (264, fixture.payer_destination),
    ] {
        data[offset..offset + 32].copy_from_slice(value.as_ref());
    }
    data[296..304].copy_from_slice(&fixture.nonce.to_le_bytes());
    data[304..312].copy_from_slice(&fixture.gross_atomic.to_le_bytes());
    data[312..320].copy_from_slice(&fixture.tax_atomic.to_le_bytes());
    data[320..328].copy_from_slice(&fixture.net_atomic.to_le_bytes());
    data[328..360].copy_from_slice(&fixture.reference_digest);
    data[360..368].copy_from_slice(&1i64.to_le_bytes());
    data[368..376].copy_from_slice(&9_999_999_999i64.to_le_bytes());
    data[408] = 1;

    Ok(json!({
        "context": {"slot": fixture.slot},
        "value": [{
            "data": [BASE64.encode(data), "base64"],
            "executable": false,
            "lamports": 1_000_000u64,
            "owner": SETTLEMENT_PROGRAM_ID,
            "rentEpoch": u64::MAX
        }]
    }))
}

fn get_refund_proof_transaction(
    state: &FixtureState,
    params: &Value,
) -> Result<Value, (i64, String)> {
    let fixture = state.refund_proof.as_ref().ok_or((
        -32601,
        "getTransaction is not configured for this fixture profile".to_string(),
    ))?;
    let values = params.as_array().ok_or((
        -32602,
        "getTransaction requires an array of parameters".to_string(),
    ))?;
    if values.first().and_then(Value::as_str) != Some(fixture.signature.as_str()) {
        return Ok(Value::Null);
    }
    let options = values
        .get(1)
        .and_then(Value::as_object)
        .ok_or((-32602, "getTransaction options are required".to_string()))?;
    if options.get("commitment").and_then(Value::as_str) != Some("finalized")
        || options.get("encoding").and_then(Value::as_str) != Some("json")
        || options
            .get("maxSupportedTransactionVersion")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err((
            -32602,
            "fixture requires finalized json transaction reads with version zero".to_string(),
        ));
    }
    let account_keys = [
        fixture.tax_vault,
        fixture.refund_source,
        fixture.payer_destination,
    ];
    let balance = |account_index: u64, owner: Pubkey, amount: u64| {
        json!({
            "accountIndex": account_index,
            "mint": fixture.mint.to_string(),
            "owner": owner.to_string(),
            "programId": SPL_TOKEN_PROGRAM_ID,
            "uiTokenAmount": {
                "amount": amount.to_string(),
                "decimals": 6,
                "uiAmountString": "0"
            }
        })
    };
    let mut event = Vec::with_capacity(328);
    event.extend_from_slice(&ADJUSTMENT_REFUND_EXECUTED_EVENT_DISCRIMINATOR);
    for value in [
        fixture.claim,
        fixture.adjustment,
        fixture.original_receipt,
        fixture.merchant,
        fixture.mint,
    ] {
        event.extend_from_slice(value.as_ref());
    }
    event.extend_from_slice(&fixture.nonce.to_le_bytes());
    event.extend_from_slice(&fixture.gross_atomic.to_le_bytes());
    event.extend_from_slice(&fixture.tax_atomic.to_le_bytes());
    event.extend_from_slice(&fixture.net_atomic.to_le_bytes());
    event.extend_from_slice(fixture.refund_funder.as_ref());
    event.extend_from_slice(fixture.payer_destination.as_ref());
    event.extend_from_slice(fixture.approver.as_ref());
    event.extend_from_slice(&fixture.reference_digest);
    Ok(json!({
        "slot": fixture.slot,
        "transaction": {
            "signatures": [fixture.signature],
            "message": {"accountKeys": account_keys.iter().map(ToString::to_string).collect::<Vec<_>>()}
        },
        "meta": {
            "err": null,
            "logMessages": [format!("Program data: {}", BASE64.encode(event))],
            "preTokenBalances": [
                balance(0, fixture.vault_authority(), fixture.tax_vault_before),
                balance(1, fixture.refund_funder, fixture.refund_source_before),
                balance(2, fixture.payer, fixture.payer_destination_before)
            ],
            "postTokenBalances": [
                balance(0, fixture.vault_authority(), fixture.tax_vault_after),
                balance(1, fixture.refund_funder, fixture.refund_source_after),
                balance(2, fixture.payer, fixture.payer_destination_after)
            ]
        }
    }))
}

fn get_refund_funding_accounts(
    fixture: &RefundFundingFixture,
    addresses: &[Value],
    values: &[Value],
) -> Result<Value, (i64, String)> {
    let expected = [
        fixture.merchant_account.as_str(),
        fixture.mint_address.as_str(),
        fixture.tax_vault.as_str(),
        fixture.refund_funder.as_str(),
        fixture.net_source.as_str(),
    ];
    if addresses.len() != expected.len()
        || addresses
            .iter()
            .zip(expected)
            .any(|(actual, expected)| actual.as_str() != Some(expected))
    {
        return Err((
            -32602,
            "fixture refund-funding account addresses do not match the configured topology"
                .to_string(),
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
    let system_account = || {
        json!({
            "data": [BASE64.encode([]), "base64"],
            "executable": false,
            "lamports": 1_000_000u64,
            "owner": "11111111111111111111111111111111",
            "rentEpoch": u64::MAX
        })
    };
    let mut mint_data = vec![0; 82];
    mint_data[44] = fixture.mint_decimals;
    let mut tax_data = vec![0; 165];
    tax_data[..32].copy_from_slice(fixture.mint.as_ref());
    let vault_authority = Pubkey::from_str(&fixture.vault_authority).map_err(|_| {
        (
            -32602,
            "fixture vault authority is not a valid public key".to_string(),
        )
    })?;
    tax_data[32..64].copy_from_slice(vault_authority.as_ref());
    let refund_funder = Pubkey::from_str(&fixture.refund_funder).map_err(|_| {
        (
            -32602,
            "fixture refund funder is not a valid public key".to_string(),
        )
    })?;
    let mut source_data = vec![0; 165];
    source_data[..32].copy_from_slice(fixture.mint.as_ref());
    source_data[32..64].copy_from_slice(refund_funder.as_ref());
    let token_account = |data: Vec<u8>| {
        json!({
            "data": [BASE64.encode(data), "base64"],
            "executable": false,
            "lamports": 1_000_000u64,
            "owner": SPL_TOKEN_PROGRAM_ID,
            "rentEpoch": u64::MAX
        })
    };
    Ok(json!({
        "context": {"slot": fixture.slot},
        "value": [
            system_account(),
            token_account(mint_data),
            token_account(tax_data),
            system_account(),
            token_account(source_data)
        ]
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
            "confirmationStatus": match observed_status {
                ObservedStatus::Confirmed => "confirmed",
                ObservedStatus::Finalized => "finalized",
                ObservedStatus::Unknown => "unknown",
            },
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

    #[test]
    fn finalized_status_is_explicit_and_not_inferred_from_submission() {
        let status = signature_status_value(
            Some("accepted-signature"),
            Some("accepted-signature"),
            ObservedStatus::Finalized,
        );
        assert_eq!(status["confirmationStatus"], "finalized");
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
            refund_funding: None,
            refund_proof: None,
            refund_funding_read_delay: Duration::ZERO,
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
