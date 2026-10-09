#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

use ed25519_dalek::SigningKey;
use jurisdiction::{
    EvidenceSource, JURISDICTION_EVIDENCE_SCHEMA_VERSION, JurisdictionEvidenceEnvelope,
    UnsignedJurisdictionEvidence,
};
use policy::{
    APPLICABILITY_ATTESTATION_SCHEMA_VERSION, ATTESTATION_SCHEMA_VERSION, ApplicabilityAttestation,
    CalculationAttestation, DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION, Digest32,
    DocumentCalculationAttestation, UnsignedApplicabilityAttestation, UnsignedAttestation,
    UnsignedDocumentAttestation, merchant_identity_digest, treatment_outcome_digest,
};
use serde_json::{Value, json};
use solana_pubkey::Pubkey;
use std::{
    env, fs,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tax_applicability::{
    APPLICABILITY_SCHEMA_V1, ApplicabilityDecision, ApplicabilityFacts, Treatment,
};
use tax_engine::{RuleSetDocument, TreatmentOutcome};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("jurisdiction-evidence") {
        return emit_jurisdiction_evidence(&args);
    }
    if args.get(1).map(String::as_str) == Some("document-attestation") {
        return emit_document_attestation(&args);
    }
    if args.get(1).map(String::as_str) == Some("applicability-attestation") {
        return emit_applicability_attestation(&args);
    }
    if args.get(1).map(String::as_str) == Some("rule-digest") {
        return emit_rule_digest(&args);
    }
    if args.len() != 6 {
        return Err("usage: attestation-fixture <quote-json> <route-mint> <merchant-id> <route-id> <seed-hex>".into());
    }

    let quote: Value = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let route_mint = Pubkey::from_str(&args[2])?;
    let merchant_id = Uuid::parse_str(&args[3])?;
    let route_id = Uuid::parse_str(&args[4])?;
    let seed = parse_seed(&args[5])?;
    let signing_key = SigningKey::from_bytes(&seed);

    let result = quote
        .get("result")
        .ok_or("quote response does not contain result")?;
    let issued_at_unix = unix_now()? - 1;
    let expires_at_unix = value_i64(&quote, "expires_at_unix")?;
    if expires_at_unix <= issued_at_unix {
        return Err("quote is already expired for the external issuer fixture".into());
    }
    let attestation = CalculationAttestation::sign(
        UnsignedAttestation {
            schema_version: ATTESTATION_SCHEMA_VERSION,
            merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
            rule_set_id: value_string(result, "rule_set_id")?,
            rule_version: value_u32(result, "rule_version")?,
            jurisdiction: value_string(result, "jurisdiction")?,
            currency: value_string(result, "currency")?,
            asset_mint: route_mint.to_bytes(),
            facts_digest: value_digest(result, "facts_digest")?,
            rule_digest: value_digest(result, "rule_digest")?,
            gross_atomic: value_amount(result, "gross_atomic")?,
            tax_atomic: value_amount(result, "total_tax_atomic")?,
            net_atomic: value_amount(result, "total_net_atomic")?,
            treatment_outcome_digest: value_treatment_outcome_digest(result)?,
            issued_at_unix,
            expires_at_unix,
            sequence: 1,
            issuer_key_id: [9; 32],
        },
        &signing_key,
    )?;

    println!(
        "{}",
        serde_json::to_string(&json!({
            "route_id": route_id,
            "attestation": attestation,
        }))?
    );
    Ok(())
}

fn emit_applicability_attestation(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 4 {
        return Err(
            "usage: attestation-fixture applicability-attestation <merchant-id> <seed-hex>".into(),
        );
    }
    let merchant_id = Uuid::parse_str(&args[2])?;
    let signing_key = SigningKey::from_bytes(&parse_seed(&args[3])?);
    let issued_at_unix = unix_now()?.saturating_sub(1);
    let expires_at_unix = issued_at_unix.saturating_add(300);
    let facts = ApplicabilityFacts {
        schema: APPLICABILITY_SCHEMA_V1.to_string(),
        product_category: "digital-service".to_string(),
        supply_type: "cross-border".to_string(),
        customer_class: "business".to_string(),
        merchant_registration: "registered".to_string(),
        customer_registration: "unregistered".to_string(),
        transaction_type: "sale".to_string(),
        evidence_digest: [6; 32],
        valid_until_unix: expires_at_unix,
    };
    let decision = ApplicabilityDecision {
        schema: APPLICABILITY_SCHEMA_V1.to_string(),
        policy_id: "policy-us-ca-digital".to_string(),
        policy_version: 3,
        policy_digest: [7; 32],
        facts_digest: facts.canonical_digest()?,
        treatment: Treatment::Taxable,
        selected_rule_id: Some("taxable-digital-service".to_string()),
        matched_rule_ids: vec!["taxable-digital-service".to_string()],
        evaluated_at_unix: issued_at_unix,
    };
    let attestation = ApplicabilityAttestation::sign(
        UnsignedApplicabilityAttestation {
            schema_version: APPLICABILITY_ATTESTATION_SCHEMA_VERSION,
            merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
            jurisdiction_resolution_digest: [2; 32],
            facts,
            decision,
            rule_set_id: "us-ca-sales-tax".to_string(),
            rule_version: 7,
            rule_digest: [3; 32],
            issued_at_unix,
            expires_at_unix,
            sequence: 9,
            issuer_key_id: [4; 32],
        },
        &signing_key,
    )?;
    println!("{}", serde_json::to_string(&attestation)?);
    Ok(())
}

fn emit_rule_digest(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 3 {
        return Err("usage: attestation-fixture rule-digest <rule-document-json>".into());
    }
    let document: RuleSetDocument = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
    document.validate()?;
    let digest = document.rule_set.canonical_digest()?;
    println!(
        "{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    Ok(())
}

fn emit_document_attestation(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 7 {
        return Err("usage: attestation-fixture document-attestation <document-quote-json> <route-mint> <merchant-id> <route-id> <seed-hex>".into());
    }
    let quote: Value = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
    let route_mint = Pubkey::from_str(&args[3])?;
    let merchant_id = Uuid::parse_str(&args[4])?;
    let route_id = Uuid::parse_str(&args[5])?;
    let seed = parse_seed(&args[6])?;
    let signing_key = SigningKey::from_bytes(&seed);
    let result = quote
        .get("result")
        .ok_or("document quote response does not contain result")?;
    let issued_at_unix = unix_now()? - 1;
    let expires_at_unix = value_i64(&quote, "expires_at_unix")?;
    if expires_at_unix <= issued_at_unix {
        return Err("document quote is already expired for the external issuer fixture".into());
    }
    let attestation = DocumentCalculationAttestation::sign(
        UnsignedDocumentAttestation {
            schema_version: DOCUMENT_CALCULATION_ATTESTATION_SCHEMA_VERSION,
            document_schema: value_string(&quote, "schema")?,
            merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
            document_id: value_string(result, "document_id")?,
            engine_version: value_string(result, "engine_version")?,
            rule_set_id: value_string(result, "rule_set_id")?,
            rule_version: value_u32(result, "rule_version")?,
            jurisdiction: value_string(result, "jurisdiction")?,
            currency: value_string(result, "currency")?,
            asset_mint: route_mint.to_bytes(),
            facts_digest: value_digest(result, "facts_digest")?,
            rule_digest: value_digest(result, "rule_digest")?,
            gross_atomic: value_amount(result, "gross_atomic")?,
            tax_atomic: value_amount(result, "total_tax_atomic")?,
            net_atomic: value_amount(result, "total_net_atomic")?,
            treatment_outcome_digest: value_treatment_outcome_digest(result)?,
            issued_at_unix,
            expires_at_unix,
            sequence: 1,
            issuer_key_id: [9; 32],
        },
        &signing_key,
    )?;
    println!(
        "{}",
        serde_json::to_string(&json!({
            "route_id": route_id,
            "attestation": attestation,
        }))?
    );
    Ok(())
}

fn emit_jurisdiction_evidence(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 5 {
        return Err("usage: attestation-fixture jurisdiction-evidence <quote-request-json> <merchant-id> <provider-seed-hex>".into());
    }
    let request: Value = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
    let facts_document = request
        .get("facts")
        .ok_or("quote request does not contain facts")?
        .clone();
    let (facts_digest, jurisdiction) = if facts_document.get("lines").is_some() {
        let facts: tax_engine::TaxDocumentFacts = serde_json::from_value(facts_document)?;
        (facts.canonical_digest()?, facts.jurisdiction)
    } else {
        let facts: tax_engine::TaxFacts = serde_json::from_value(facts_document)?;
        (facts.canonical_digest()?, facts.jurisdiction)
    };
    let merchant_id = Uuid::parse_str(&args[3])?;
    let signing_key = SigningKey::from_bytes(&parse_seed(&args[4])?);
    let provider_key_id = match env::var("ATTESTATION_PROVIDER_KEY_ID_HEX") {
        Ok(value) => parse_seed(&value)?,
        Err(_) => [4; 32],
    };
    let issued_at_unix = unix_now()? - 1;
    let envelope = JurisdictionEvidenceEnvelope::sign(
        UnsignedJurisdictionEvidence {
            schema_version: JURISDICTION_EVIDENCE_SCHEMA_VERSION,
            merchant_id: merchant_identity_digest(merchant_id.as_bytes()),
            facts_digest,
            jurisdiction_code: jurisdiction,
            source: EvidenceSource::ShippingDestination,
            evidence_digest: [3; 32],
            issued_at_unix,
            expires_at_unix: issued_at_unix + 300,
            provider_key_id,
        },
        &signing_key,
    )?;
    println!("{}", serde_json::to_string(&envelope)?);
    Ok(())
}

fn unix_now() -> Result<i64, Box<dyn std::error::Error>> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )?)
}

fn value_string(value: &Value, field: &str) -> Result<String, Box<dyn std::error::Error>> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("quote field {field} must be a string").into())
}

fn value_i64(value: &Value, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("quote field {field} must be an integer").into())
}

fn value_u32(value: &Value, field: &str) -> Result<u32, Box<dyn std::error::Error>> {
    u32::try_from(value_i64(value, field)?)
        .map_err(|_| format!("quote field {field} is out of range").into())
}

fn value_amount(value: &Value, field: &str) -> Result<u64, Box<dyn std::error::Error>> {
    value_string(value, field)?
        .parse()
        .map_err(|_| format!("quote field {field} is not a u64 string").into())
}

fn value_treatment_outcome_digest(value: &Value) -> Result<Digest32, Box<dyn std::error::Error>> {
    let outcome: TreatmentOutcome = serde_json::from_value(
        value
            .get("treatment_outcome")
            .ok_or("quote field treatment_outcome is required")?
            .clone(),
    )?;
    Ok(treatment_outcome_digest(
        outcome.treatment,
        outcome.arithmetic_tax_atomic,
        outcome.collected_tax_atomic,
        outcome.self_assessed_tax_atomic,
        outcome.withheld_tax_atomic,
        outcome.taxable_base_atomic,
    ))
}

fn value_digest(value: &Value, field: &str) -> Result<Digest32, Box<dyn std::error::Error>> {
    let bytes = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("quote field {field} must be a byte array"))?;
    if bytes.len() != 32 {
        return Err(format!("quote field {field} must contain 32 bytes").into());
    }
    let mut digest = [0u8; 32];
    for (index, byte) in bytes.iter().enumerate() {
        digest[index] = u8::try_from(byte.as_u64().ok_or("digest byte must be an integer")?)?;
    }
    Ok(digest)
}

fn parse_seed(value: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    if value.len() != 64 {
        return Err("seed must contain exactly 64 hexadecimal characters".into());
    }
    let mut seed = [0u8; 32];
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err("seed must contain complete hexadecimal byte pairs".into());
    }
    for (index, pair) in pairs.iter().enumerate() {
        seed[index] = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
    }
    Ok(seed)
}

fn hex_digit(value: u8) -> Result<u8, Box<dyn std::error::Error>> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("seed contains a non-hexadecimal character".into()),
    }
}
