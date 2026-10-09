#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Deterministic document-to-payment mapping.
//!
//! This bounded context intentionally receives an already calculated,
//! server-owned document result. It never prorates or recalculates tax. A
//! caller that needs a partial line or quantity payment must obtain a new
//! document quote whose facts describe that payment.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use tax_engine::{AtomicAmount, MAX_DOCUMENT_LINES, TaxDocumentResult, TaxLineResult};
use thiserror::Error;

pub type Digest32 = [u8; 32];
pub const DOCUMENT_PAYMENT_SCOPE_SCHEMA_V1: &str = "solana-tax-protocol/document-payment-scope/v1";
pub const MAX_SCOPE_LINE_IDS: usize = MAX_DOCUMENT_LINES;
pub const MAX_LINE_ID_BYTES: usize = 64;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaymentScope {
    FullDocument,
    LineSubset { line_ids: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentPaymentAuthorization {
    pub source_attestation_digest: Digest32,
    pub payment_scope_digest: Digest32,
    pub scope: PaymentScope,
    pub gross_atomic: u64,
    pub tax_atomic: u64,
    pub net_atomic: u64,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum DocumentPaymentError {
    #[error("source document attestation digest must not be zero")]
    EmptySourceAttestationDigest,
    #[error("document result schema or identity is invalid")]
    InvalidDocumentResult,
    #[error("payment scope must contain at least one line")]
    EmptyLineSubset,
    #[error("payment scope contains too many lines")]
    LineSubsetTooLarge,
    #[error("payment scope line ID is empty or too long")]
    InvalidLineId,
    #[error("payment scope contains a duplicate line ID")]
    DuplicateLineId,
    #[error("payment scope contains an unknown line ID")]
    UnknownLineId,
    #[error("a complete line subset must use FullDocument scope")]
    UseFullDocumentScope,
    #[error("document amount is outside Solana u64 range")]
    AmountOutOfRange,
    #[error("document amount conservation failed")]
    AmountConservationViolation,
    #[error("document arithmetic overflow")]
    ArithmeticOverflow,
}

/// Derives the server-owned payment amounts and scope commitment. The
/// function is pure and bounded by the tax engine's document line limit.
pub fn authorize_document_payment(
    result: &TaxDocumentResult,
    source_attestation_digest: Digest32,
    scope: PaymentScope,
) -> Result<DocumentPaymentAuthorization, DocumentPaymentError> {
    if source_attestation_digest == [0; 32] {
        return Err(DocumentPaymentError::EmptySourceAttestationDigest);
    }
    validate_document_result(result)?;
    let (gross_atomic, tax_atomic, net_atomic, canonical_scope) = match scope {
        PaymentScope::FullDocument => (
            to_u64(result.gross_atomic)?,
            to_u64(result.total_tax_atomic)?,
            to_u64(result.total_net_atomic)?,
            PaymentScope::FullDocument,
        ),
        PaymentScope::LineSubset { line_ids } => {
            let ids = canonical_line_ids(line_ids, result.lines.len())?;
            let selected = ids
                .iter()
                .map(|id| {
                    result
                        .lines
                        .iter()
                        .find(|line| line.id == *id)
                        .ok_or(DocumentPaymentError::UnknownLineId)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let gross = sum_lines(&selected, |line| line.gross_atomic)?;
            let tax = sum_lines(&selected, |line| line.total_tax_atomic)?;
            let net = sum_lines(&selected, |line| line.total_net_atomic)?;
            (
                to_u64(gross)?,
                to_u64(tax)?,
                to_u64(net)?,
                PaymentScope::LineSubset { line_ids: ids },
            )
        }
    };
    if gross_atomic == 0 || tax_atomic.checked_add(net_atomic) != Some(gross_atomic) {
        return Err(DocumentPaymentError::AmountConservationViolation);
    }
    let payment_scope_digest =
        payment_scope_digest(result, source_attestation_digest, &canonical_scope)?;
    Ok(DocumentPaymentAuthorization {
        source_attestation_digest,
        payment_scope_digest,
        scope: canonical_scope,
        gross_atomic,
        tax_atomic,
        net_atomic,
    })
}

fn validate_document_result(result: &TaxDocumentResult) -> Result<(), DocumentPaymentError> {
    if result.document_id.trim().is_empty()
        || result.document_id.len() > MAX_LINE_ID_BYTES
        || result.lines.is_empty()
        || result.lines.len() > MAX_DOCUMENT_LINES
    {
        return Err(DocumentPaymentError::InvalidDocumentResult);
    }
    let gross = to_u64(result.gross_atomic)?;
    let tax = to_u64(result.total_tax_atomic)?;
    let net = to_u64(result.total_net_atomic)?;
    if gross == 0 || tax.checked_add(net) != Some(gross) {
        return Err(DocumentPaymentError::AmountConservationViolation);
    }
    let line_gross = sum_lines(&result.lines.iter().collect::<Vec<_>>(), |line| {
        line.gross_atomic
    })?;
    let line_tax = sum_lines(&result.lines.iter().collect::<Vec<_>>(), |line| {
        line.total_tax_atomic
    })?;
    let line_net = sum_lines(&result.lines.iter().collect::<Vec<_>>(), |line| {
        line.total_net_atomic
    })?;
    if to_u64(line_gross)? != gross || to_u64(line_tax)? != tax || to_u64(line_net)? != net {
        return Err(DocumentPaymentError::InvalidDocumentResult);
    }
    Ok(())
}

fn canonical_line_ids(
    line_ids: Vec<String>,
    document_line_count: usize,
) -> Result<Vec<String>, DocumentPaymentError> {
    if line_ids.is_empty() {
        return Err(DocumentPaymentError::EmptyLineSubset);
    }
    if line_ids.len() > MAX_SCOPE_LINE_IDS {
        return Err(DocumentPaymentError::LineSubsetTooLarge);
    }
    let mut ids = line_ids;
    for id in &ids {
        if id.trim().is_empty() || id.len() > MAX_LINE_ID_BYTES {
            return Err(DocumentPaymentError::InvalidLineId);
        }
    }
    ids.sort();
    let unique = ids.iter().cloned().collect::<BTreeSet<_>>();
    if unique.len() != ids.len() {
        return Err(DocumentPaymentError::DuplicateLineId);
    }
    if unique.len() == document_line_count {
        return Err(DocumentPaymentError::UseFullDocumentScope);
    }
    Ok(ids)
}

fn sum_lines(
    lines: &[&TaxLineResult],
    amount: impl Fn(&TaxLineResult) -> AtomicAmount,
) -> Result<AtomicAmount, DocumentPaymentError> {
    lines.iter().try_fold(0_i128, |total, line| {
        total
            .checked_add(amount(line))
            .ok_or(DocumentPaymentError::ArithmeticOverflow)
    })
}

fn to_u64(value: AtomicAmount) -> Result<u64, DocumentPaymentError> {
    u64::try_from(value).map_err(|_| DocumentPaymentError::AmountOutOfRange)
}

pub fn payment_scope_digest(
    result: &TaxDocumentResult,
    source_attestation_digest: Digest32,
    scope: &PaymentScope,
) -> Result<Digest32, DocumentPaymentError> {
    let mut bytes = Vec::with_capacity(256);
    bytes.extend_from_slice(DOCUMENT_PAYMENT_SCOPE_SCHEMA_V1.as_bytes());
    put_text(&mut bytes, tax_engine::TAX_DOCUMENT_SCHEMA_V1);
    put_text(&mut bytes, &result.document_id);
    bytes.extend_from_slice(&result.facts_digest);
    bytes.extend_from_slice(&source_attestation_digest);
    match scope {
        PaymentScope::FullDocument => bytes.push(0),
        PaymentScope::LineSubset { line_ids } => {
            bytes.push(1);
            bytes.extend_from_slice(&(line_ids.len() as u32).to_be_bytes());
            for id in line_ids {
                put_text(&mut bytes, id);
            }
        }
    }
    Ok(Sha256::digest(bytes).into())
}

fn put_text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tax_engine::{ComponentResult, TaxLineResult, Treatment, TreatmentOutcome};

    fn digest(value: u8) -> Digest32 {
        [value; 32]
    }

    fn result() -> TaxDocumentResult {
        let line = |id: &str, gross: i128, tax: i128| TaxLineResult {
            id: id.to_string(),
            gross_atomic: gross,
            total_tax_atomic: tax,
            total_net_atomic: gross - tax,
            components: vec![ComponentResult {
                id: "sales-tax".to_string(),
                label: "Sales tax".to_string(),
                base_atomic: gross - tax,
                effective_base_atomic: gross - tax,
                threshold_atomic: None,
                exemption_applied: false,
                raw_tax: tax_engine::RationalParts {
                    numerator: tax,
                    denominator: 1,
                },
                credit_atomic: 0,
                clamped_tax_atomic: tax,
                tax_atomic: tax,
                explanation: "test".to_string(),
            }],
        };
        TaxDocumentResult {
            document_id: "invoice-1".to_string(),
            rule_set_id: "rule-1".to_string(),
            rule_version: 1,
            engine_version: "test".to_string(),
            atomic_decimals: 2,
            currency: "USD".to_string(),
            jurisdiction: "US-CA".to_string(),
            jurisdiction_resolution: None,
            fx_conversion: None,
            applicability: None,
            facts_digest: digest(9),
            rule_digest: digest(8),
            gross_atomic: 1500,
            total_tax_atomic: 150,
            total_net_atomic: 1350,
            treatment_outcome: TreatmentOutcome {
                treatment: Treatment::Taxable,
                arithmetic_tax_atomic: 150,
                collected_tax_atomic: 150,
                self_assessed_tax_atomic: 0,
                withheld_tax_atomic: 0,
                taxable_base_atomic: 1500,
            },
            lines: vec![line("a", 1000, 100), line("b", 500, 50)],
        }
    }

    #[test]
    fn full_document_uses_exact_totals() {
        let authorization =
            authorize_document_payment(&result(), digest(7), PaymentScope::FullDocument).unwrap();
        assert_eq!(
            (
                authorization.gross_atomic,
                authorization.tax_atomic,
                authorization.net_atomic
            ),
            (1500, 150, 1350)
        );
        assert_eq!(authorization.scope, PaymentScope::FullDocument);
    }

    #[test]
    fn line_subset_sums_server_owned_line_results() {
        let authorization = authorize_document_payment(
            &result(),
            digest(7),
            PaymentScope::LineSubset {
                line_ids: vec!["b".into()],
            },
        )
        .unwrap();
        assert_eq!(
            (
                authorization.gross_atomic,
                authorization.tax_atomic,
                authorization.net_atomic
            ),
            (500, 50, 450)
        );
        assert_eq!(
            authorization.scope,
            PaymentScope::LineSubset {
                line_ids: vec!["b".into()]
            }
        );
    }

    #[test]
    fn scope_is_canonical_and_rejects_duplicates_unknowns_and_full_subset() {
        let document = result();
        let first = authorize_document_payment(
            &document,
            digest(7),
            PaymentScope::LineSubset {
                line_ids: vec!["b".into(), "a".into()],
            },
        );
        assert_eq!(
            first.unwrap_err(),
            DocumentPaymentError::UseFullDocumentScope
        );
        let duplicate = authorize_document_payment(
            &document,
            digest(7),
            PaymentScope::LineSubset {
                line_ids: vec!["a".into(), "a".into()],
            },
        );
        assert_eq!(
            duplicate.unwrap_err(),
            DocumentPaymentError::DuplicateLineId
        );
        let unknown = authorize_document_payment(
            &document,
            digest(7),
            PaymentScope::LineSubset {
                line_ids: vec!["missing".into()],
            },
        );
        assert_eq!(unknown.unwrap_err(), DocumentPaymentError::UnknownLineId);
    }

    #[test]
    fn changing_source_attestation_changes_scope_digest() {
        let document = result();
        let one =
            authorize_document_payment(&document, digest(7), PaymentScope::FullDocument).unwrap();
        let two =
            authorize_document_payment(&document, digest(6), PaymentScope::FullDocument).unwrap();
        assert_ne!(one.payment_scope_digest, two.payment_scope_digest);
    }
}
