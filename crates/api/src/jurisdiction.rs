use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::VerifyingKey;
use jurisdiction::{
    Digest32, JurisdictionCandidate, JurisdictionError, JurisdictionEvidenceEnvelope,
    MAX_CANDIDATES, candidate_set_digest,
};
use policy::merchant_identity_digest;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

const MAX_PROVIDER_KEYS: usize = 32;

#[derive(Clone, Debug)]
pub struct JurisdictionProviderRegistry {
    keys: Arc<HashMap<Digest32, VerifyingKey>>,
}

#[derive(Debug, Error)]
pub enum ProviderRegistryError {
    #[error("jurisdiction provider key registry is empty")]
    Empty,
    #[error("jurisdiction provider key registry is invalid: {0}")]
    Invalid(String),
    #[error("jurisdiction provider key is unknown")]
    UnknownKey,
    #[error("jurisdiction provider evidence is invalid: {0}")]
    Evidence(#[from] JurisdictionError),
}

#[derive(Debug, Deserialize)]
struct RegistryDocument {
    keys: Vec<RegistryKey>,
}

#[derive(Debug, Deserialize)]
struct RegistryKey {
    key_id: String,
    public_key: String,
}

impl JurisdictionProviderRegistry {
    pub fn from_json(document: &str) -> Result<Self, ProviderRegistryError> {
        let document: RegistryDocument = serde_json::from_str(document)
            .map_err(|error| ProviderRegistryError::Invalid(format!("JSON: {error}")))?;
        if document.keys.is_empty() || document.keys.len() > MAX_PROVIDER_KEYS {
            return Err(ProviderRegistryError::Empty);
        }
        let mut keys = HashMap::with_capacity(document.keys.len());
        for entry in document.keys {
            let key_id = decode_hex_digest(&entry.key_id)?;
            let public_key_bytes = URL_SAFE_NO_PAD
                .decode(entry.public_key.as_bytes())
                .map_err(|_| {
                    ProviderRegistryError::Invalid("public_key must be base64url".to_string())
                })?;
            let public_key: [u8; 32] = public_key_bytes.try_into().map_err(|_| {
                ProviderRegistryError::Invalid("public_key must contain 32 bytes".to_string())
            })?;
            let verifying_key = VerifyingKey::from_bytes(&public_key)
                .map_err(|_| ProviderRegistryError::Invalid("public_key is invalid".to_string()))?;
            if keys.insert(key_id, verifying_key).is_some() {
                return Err(ProviderRegistryError::Invalid(
                    "duplicate provider key_id".to_string(),
                ));
            }
        }
        Ok(Self {
            keys: Arc::new(keys),
        })
    }

    pub fn verify(
        &self,
        envelopes: &[JurisdictionEvidenceEnvelope],
        merchant_id: Uuid,
        facts_digest: Digest32,
        now_unix: i64,
    ) -> Result<Vec<JurisdictionCandidate>, ProviderRegistryError> {
        if envelopes.is_empty() || envelopes.len() > MAX_CANDIDATES {
            return Err(ProviderRegistryError::Evidence(
                JurisdictionError::InvalidCandidate(
                    "evidence count is outside the supported bound",
                ),
            ));
        }
        let merchant_digest = merchant_identity_digest(merchant_id.as_bytes());
        let mut candidates = Vec::with_capacity(envelopes.len());
        for envelope in envelopes {
            let key = self
                .keys
                .get(&envelope.unsigned.provider_key_id)
                .ok_or(ProviderRegistryError::UnknownKey)?;
            candidates.push(envelope.verify_with_key_at(
                key,
                merchant_digest,
                facts_digest,
                now_unix,
            )?);
        }
        candidate_set_digest(&candidates)?;
        Ok(candidates)
    }
}

fn decode_hex_digest(value: &str) -> Result<Digest32, ProviderRegistryError> {
    if value.len() != 64 {
        return Err(ProviderRegistryError::Invalid(
            "key_id must contain 64 hexadecimal characters".to_string(),
        ));
    }
    let mut digest = [0_u8; 32];
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (index, pair) in pairs.iter().enumerate() {
        let high = hex_nibble(pair[0]).ok_or_else(|| {
            ProviderRegistryError::Invalid("key_id must be hexadecimal".to_string())
        })?;
        let low = hex_nibble(pair[1]).ok_or_else(|| {
            ProviderRegistryError::Invalid("key_id must be hexadecimal".to_string())
        })?;
        digest[index] = (high << 4) | low;
    }
    if digest == [0; 32] {
        return Err(ProviderRegistryError::Invalid(
            "key_id must be non-zero".to_string(),
        ));
    }
    Ok(digest)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use jurisdiction::{
        EvidenceSource, JURISDICTION_EVIDENCE_SCHEMA_VERSION, JurisdictionEvidenceEnvelope,
        UnsignedJurisdictionEvidence,
    };

    #[test]
    fn verifies_only_registered_provider_evidence_bound_to_merchant_and_facts() {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let public_key = URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes());
        let registry = JurisdictionProviderRegistry::from_json(&format!(
            "{{\"keys\":[{{\"key_id\":\"{}\",\"public_key\":\"{}\"}}]}}",
            "04".repeat(32),
            public_key
        ))
        .unwrap();
        let envelope = JurisdictionEvidenceEnvelope::sign(
            UnsignedJurisdictionEvidence {
                schema_version: JURISDICTION_EVIDENCE_SCHEMA_VERSION,
                merchant_id: merchant_identity_digest(&[1; 16]),
                facts_digest: [2; 32],
                jurisdiction_code: "US-CA".to_string(),
                source: EvidenceSource::ShippingDestination,
                evidence_digest: [3; 32],
                issued_at_unix: 1_800_000_000,
                expires_at_unix: 1_800_000_300,
                provider_key_id: [4; 32],
            },
            &signing_key,
        )
        .unwrap();
        let merchant = Uuid::from_bytes([1; 16]);
        let candidates = registry.verify(&[envelope], merchant, [2; 32], 1_800_000_100);
        assert_eq!(candidates.unwrap()[0].jurisdiction_code, "US-CA");
    }
}
