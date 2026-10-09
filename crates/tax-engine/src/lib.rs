#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Deterministic, replayable tax calculation domain logic.
//!
//! This crate intentionally has no network, clock, database, filesystem, RPC,
//! or wallet dependencies. All external facts are supplied as immutable input.

use jurisdiction::JurisdictionResolution;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use tax_applicability::ApplicabilityDecision;
pub use tax_applicability::Treatment;
use thiserror::Error;

pub type AtomicAmount = i128;
pub type Digest32 = [u8; 32];
pub const MAX_ATOMIC_AMOUNT: AtomicAmount = u64::MAX as AtomicAmount;
pub const MAX_COMPONENTS: usize = 128;
pub const MAX_DOCUMENT_LINES: usize = 256;
pub const MAX_EXEMPTION_CODES: usize = 32;
pub const MAX_CODE_BYTES: usize = 64;
pub const MAX_PERIOD_ID_BYTES: usize = 128;
pub const MAX_CURRENCY_CODE_BYTES: usize = 16;
pub const RULE_SET_DOCUMENT_SCHEMA_V1: &str = "solana-tax-protocol/tax-rule/v1";
pub const RULE_SET_DOCUMENT_SCHEMA_V2: &str = "solana-tax-protocol/tax-rule/v2";
pub const TAX_DOCUMENT_SCHEMA_V1: &str = "solana-tax-protocol/tax-document/v1";
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod serde_i128_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse::<i128>().map_err(serde::de::Error::custom)
    }
}

pub mod serde_option_i128_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<i128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<i128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        value
            .map(|value| value.parse::<i128>().map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RoundingMode {
    Floor,
    Ceil,
    HalfUp,
    HalfEven,
    AwayFromZero,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum RoundingScope {
    #[default]
    Component,
    Settlement,
    Line,
    Invoice,
}

fn is_component_scope(scope: &RoundingScope) -> bool {
    *scope == RoundingScope::Component
}

fn default_rounding_quantum() -> AtomicAmount {
    1
}

fn is_default_rounding_quantum(quantum: &AtomicAmount) -> bool {
    *quantum == 1
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoundingSpec {
    pub mode: RoundingMode,
    #[serde(default, skip_serializing_if = "is_component_scope")]
    pub scope: RoundingScope,
    #[serde(
        default = "default_rounding_quantum",
        skip_serializing_if = "is_default_rounding_quantum",
        with = "serde_i128_string"
    )]
    pub quantum_atomic: AtomicAmount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(numerator: i128, denominator: i128) -> Result<Self, TaxError> {
        if denominator <= 0 {
            return Err(TaxError::InvalidRule("denominator must be positive"));
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    fn from_integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    fn checked_add(self, other: Self) -> Result<Self, TaxError> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        Self::new(
            left.checked_add(right)
                .ok_or(TaxError::ArithmeticOverflow)?,
            denominator,
        )
    }

    fn checked_sub(self, other: Self) -> Result<Self, TaxError> {
        self.checked_add(Self::new(
            other
                .numerator
                .checked_neg()
                .ok_or(TaxError::ArithmeticOverflow)?,
            other.denominator,
        )?)
    }

    fn checked_mul(self, other: Self) -> Result<Self, TaxError> {
        Self::new(
            self.numerator
                .checked_mul(other.numerator)
                .ok_or(TaxError::ArithmeticOverflow)?,
            self.denominator
                .checked_mul(other.denominator)
                .ok_or(TaxError::ArithmeticOverflow)?,
        )
    }

    fn checked_div(self, other: Self) -> Result<Self, TaxError> {
        if other.numerator == 0 {
            return Err(TaxError::ArithmeticOverflow);
        }
        let (numerator, denominator) = if other.numerator < 0 {
            (
                self.numerator
                    .checked_mul(
                        other
                            .denominator
                            .checked_neg()
                            .ok_or(TaxError::ArithmeticOverflow)?,
                    )
                    .ok_or(TaxError::ArithmeticOverflow)?,
                self.denominator
                    .checked_mul(
                        other
                            .numerator
                            .checked_neg()
                            .ok_or(TaxError::ArithmeticOverflow)?,
                    )
                    .ok_or(TaxError::ArithmeticOverflow)?,
            )
        } else {
            (
                self.numerator
                    .checked_mul(other.denominator)
                    .ok_or(TaxError::ArithmeticOverflow)?,
                self.denominator
                    .checked_mul(other.numerator)
                    .ok_or(TaxError::ArithmeticOverflow)?,
            )
        };
        Self::new(numerator, denominator)
    }

    fn checked_mul_integer(self, value: i128) -> Result<Self, TaxError> {
        Self::new(
            self.numerator
                .checked_mul(value)
                .ok_or(TaxError::ArithmeticOverflow)?,
            self.denominator,
        )
    }

    fn is_negative(self) -> bool {
        self.numerator < 0
    }

    fn is_less_than(self, other: Self) -> Result<bool, TaxError> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        Ok(left < right)
    }

    fn is_greater_than(self, other: Self) -> Result<bool, TaxError> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or(TaxError::ArithmeticOverflow)?;
        Ok(left > right)
    }

    fn round(self, mode: RoundingMode) -> Result<i128, TaxError> {
        if self.denominator <= 0 {
            return Err(TaxError::ArithmeticOverflow);
        }
        let quotient = self.numerator / self.denominator;
        let remainder = self.numerator % self.denominator;
        if remainder == 0 {
            return Ok(quotient);
        }

        let absolute_remainder = remainder
            .checked_abs()
            .ok_or(TaxError::ArithmeticOverflow)?;
        let absolute_denominator = self.denominator;
        let increment = match mode {
            RoundingMode::Floor if self.numerator < 0 => -1,
            RoundingMode::Floor => 0,
            RoundingMode::Ceil if self.numerator > 0 => 1,
            RoundingMode::Ceil => 0,
            RoundingMode::AwayFromZero => {
                if self.numerator > 0 {
                    1
                } else {
                    -1
                }
            }
            RoundingMode::HalfUp | RoundingMode::HalfEven => {
                let doubled = absolute_remainder
                    .checked_mul(2)
                    .ok_or(TaxError::ArithmeticOverflow)?;
                if doubled > absolute_denominator
                    || (doubled == absolute_denominator
                        && (mode == RoundingMode::HalfUp || quotient % 2 != 0))
                {
                    if self.numerator > 0 { 1 } else { -1 }
                } else {
                    0
                }
            }
        };
        quotient
            .checked_add(increment)
            .ok_or(TaxError::ArithmeticOverflow)
    }

    fn round_to_quantum(
        self,
        mode: RoundingMode,
        quantum_atomic: AtomicAmount,
    ) -> Result<i128, TaxError> {
        if quantum_atomic <= 0 {
            return Err(TaxError::InvalidRule("rounding quantum must be positive"));
        }
        let quantum = Rational::from_integer(quantum_atomic);
        self.checked_div(quantum)?
            .round(mode)?
            .checked_mul(quantum_atomic)
            .ok_or(TaxError::ArithmeticOverflow)
    }

    fn parts(self) -> RationalParts {
        RationalParts {
            numerator: self.numerator,
            denominator: self.denominator,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RationalParts {
    #[serde(with = "serde_i128_string")]
    pub numerator: i128,
    #[serde(with = "serde_i128_string")]
    pub denominator: i128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaxFacts {
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub net_atomic: AtomicAmount,
    /// Number of decimal places represented by one whole unit of the asset.
    /// This is metadata only; settlement binds it to the finalized mint route.
    pub atomic_decimals: u8,
    pub currency: String,
    pub jurisdiction: String,
    /// Server-owned resolution evidence. `None` is retained only for legacy
    /// records; new globally supported quote paths must require `Some`.
    #[serde(default)]
    pub jurisdiction_resolution: Option<JurisdictionResolution>,
    /// Raw amounts are in `source_currency` when present; `currency` is the
    /// target calculation/settlement currency.
    #[serde(default)]
    pub fx_conversion: Option<FxConversionEvidence>,
    #[serde(default)]
    pub exemption_codes: Vec<String>,
    #[serde(default)]
    pub cumulative_period: Option<CumulativePeriodSnapshot>,
    /// Evidence-bound product/tax treatment. `None` is retained only for
    /// legacy calculations; new applicability-aware paths must provide it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<ApplicabilityDecision>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CumulativePeriodSnapshot {
    pub period_id: String,
    #[serde(with = "serde_i128_string")]
    pub cumulative_base_atomic: AtomicAmount,
    pub evidence_digest: Digest32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FxConversionEvidence {
    pub source_currency: String,
    pub target_currency: String,
    #[serde(with = "serde_i128_string")]
    pub numerator: i128,
    #[serde(with = "serde_i128_string")]
    pub denominator: i128,
    pub rounding_mode: RoundingMode,
    #[serde(with = "serde_i128_string")]
    pub quantum_atomic: AtomicAmount,
    pub provider: String,
    pub quote_id: String,
    pub observed_at_unix: i64,
    pub evidence_digest: Digest32,
}

impl FxConversionEvidence {
    fn validate_for_target(&self, target_currency: &str) -> Result<(), TaxError> {
        for currency in [&self.source_currency, &self.target_currency] {
            if currency.trim().is_empty() || currency.len() > MAX_CURRENCY_CODE_BYTES {
                return Err(TaxError::InvalidFacts(
                    "FX currency code is required and bounded",
                ));
            }
        }
        if self.source_currency == self.target_currency {
            return Err(TaxError::InvalidFacts(
                "FX source and target currencies must differ",
            ));
        }
        if self.target_currency != target_currency {
            return Err(TaxError::InvalidFacts(
                "FX target currency must match facts currency",
            ));
        }
        if self.numerator <= 0 || self.denominator <= 0 {
            return Err(TaxError::InvalidFacts(
                "FX rate must be positive with a positive denominator",
            ));
        }
        if self.quantum_atomic <= 0 || self.quantum_atomic > MAX_ATOMIC_AMOUNT {
            return Err(TaxError::InvalidFacts(
                "FX rounding quantum must be a positive bounded atomic amount",
            ));
        }
        if self.provider.trim().is_empty()
            || self.provider.len() > MAX_CODE_BYTES
            || self.quote_id.trim().is_empty()
            || self.quote_id.len() > MAX_CODE_BYTES
        {
            return Err(TaxError::InvalidFacts(
                "FX provider and quote identity are invalid",
            ));
        }
        if self.observed_at_unix <= 0 || self.evidence_digest == [0; 32] {
            return Err(TaxError::InvalidFacts("FX observation evidence is invalid"));
        }
        Ok(())
    }

    fn convert_amount(&self, amount_atomic: AtomicAmount) -> Result<AtomicAmount, TaxError> {
        if !(0..=MAX_ATOMIC_AMOUNT).contains(&amount_atomic) {
            return Err(TaxError::InvalidFacts(
                "FX source amount must fit Solana atomic units",
            ));
        }
        let converted = Rational::from_integer(amount_atomic)
            .checked_mul(Rational::new(self.numerator, self.denominator)?)?
            .round_to_quantum(self.rounding_mode, self.quantum_atomic)?;
        if !(0..=MAX_ATOMIC_AMOUNT).contains(&converted) {
            return Err(TaxError::FxConversionOutOfRange);
        }
        Ok(converted)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaxLineFacts {
    pub id: String,
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub net_atomic: AtomicAmount,
    #[serde(default)]
    pub exemption_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaxDocumentFacts {
    pub id: String,
    pub atomic_decimals: u8,
    pub currency: String,
    pub jurisdiction: String,
    #[serde(default)]
    pub jurisdiction_resolution: Option<JurisdictionResolution>,
    #[serde(default)]
    pub fx_conversion: Option<FxConversionEvidence>,
    #[serde(default)]
    pub cumulative_period: Option<CumulativePeriodSnapshot>,
    /// Document-level applicability context. It binds to the document facts
    /// digest, not to any individual line's arithmetic facts digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<ApplicabilityDecision>,
    pub lines: Vec<TaxLineFacts>,
}

impl TaxDocumentFacts {
    pub fn validate(&self) -> Result<(), TaxError> {
        if self.id.trim().is_empty() || self.id.len() > MAX_CODE_BYTES {
            return Err(TaxError::InvalidFacts(
                "document id is required and must be bounded",
            ));
        }
        if self.lines.is_empty() || self.lines.len() > MAX_DOCUMENT_LINES {
            return Err(TaxError::InvalidFacts(
                "document line count is outside the supported bound",
            ));
        }
        TaxFacts {
            gross_atomic: 0,
            net_atomic: 0,
            atomic_decimals: self.atomic_decimals,
            currency: self.currency.clone(),
            jurisdiction: self.jurisdiction.clone(),
            jurisdiction_resolution: self.jurisdiction_resolution.clone(),
            fx_conversion: self.fx_conversion.clone(),
            exemption_codes: Vec::new(),
            cumulative_period: self.cumulative_period.clone(),
            applicability: None,
        }
        .validate()?;

        let mut ids = HashMap::new();
        let mut gross_total = 0_i128;
        let mut net_total = 0_i128;
        for line in &self.lines {
            if line.id.trim().is_empty() || line.id.len() > MAX_CODE_BYTES {
                return Err(TaxError::InvalidFacts(
                    "line id is required and must be bounded",
                ));
            }
            if ids.insert(line.id.as_str(), true).is_some() {
                return Err(TaxError::InvalidFacts("line ids must be unique"));
            }
            if line.gross_atomic <= 0 || line.net_atomic <= 0 {
                return Err(TaxError::InvalidFacts(
                    "line gross and net amounts must be positive",
                ));
            }
            if line.gross_atomic > MAX_ATOMIC_AMOUNT || line.net_atomic > MAX_ATOMIC_AMOUNT {
                return Err(TaxError::InvalidFacts(
                    "line amounts must fit Solana u64 atomic units",
                ));
            }
            if line.net_atomic > line.gross_atomic {
                return Err(TaxError::InvalidFacts(
                    "line net amount cannot exceed gross amount",
                ));
            }
            if line.exemption_codes.len() > MAX_EXEMPTION_CODES {
                return Err(TaxError::InvalidFacts("too many line exemption codes"));
            }
            for code in &line.exemption_codes {
                if code.trim().is_empty() || code.len() > MAX_CODE_BYTES {
                    return Err(TaxError::InvalidFacts("invalid line exemption code"));
                }
            }
            gross_total = gross_total
                .checked_add(line.gross_atomic)
                .ok_or(TaxError::ArithmeticOverflow)?;
            net_total = net_total
                .checked_add(line.net_atomic)
                .ok_or(TaxError::ArithmeticOverflow)?;
        }
        if gross_total > MAX_ATOMIC_AMOUNT || net_total > MAX_ATOMIC_AMOUNT {
            return Err(TaxError::InvalidFacts(
                "document totals must fit Solana u64 atomic units",
            ));
        }
        if let Some(applicability) = &self.applicability {
            validate_applicability_binding(
                applicability,
                self.pre_applicability_digest_without_validation()?,
            )?;
        }
        Ok(())
    }

    pub fn pre_applicability_digest(&self) -> Result<Digest32, TaxError> {
        let mut without_applicability = self.clone();
        without_applicability.applicability = None;
        without_applicability.validate()?;
        without_applicability.pre_applicability_digest_without_validation()
    }

    fn pre_applicability_digest_without_validation(&self) -> Result<Digest32, TaxError> {
        let mut encoder = CanonicalEncoder::new();
        self.encode_base(&mut encoder)?;
        Ok(encoder.digest())
    }

    pub fn canonical_digest(&self) -> Result<Digest32, TaxError> {
        self.validate()?;
        let mut encoder = CanonicalEncoder::new();
        self.encode_base(&mut encoder)?;
        encode_optional_applicability(&mut encoder, &self.applicability)?;
        Ok(encoder.digest())
    }

    fn encode_base(&self, encoder: &mut CanonicalEncoder) -> Result<(), TaxError> {
        encoder.text(TAX_DOCUMENT_SCHEMA_V1)?;
        encoder.text(&self.id)?;
        encoder.u8(self.atomic_decimals);
        encoder.text(&self.currency)?;
        encoder.text(&self.jurisdiction)?;
        let mut lines = self.lines.iter().collect::<Vec<_>>();
        lines.sort_by(|left, right| left.id.cmp(&right.id));
        encoder.count(lines.len())?;
        for line in lines {
            encoder.text(&line.id)?;
            encoder.i128(line.gross_atomic);
            encoder.i128(line.net_atomic);
            let mut exemption_codes = line.exemption_codes.iter().collect::<Vec<_>>();
            exemption_codes.sort();
            encoder.count(exemption_codes.len())?;
            for code in exemption_codes {
                encoder.text(code)?;
            }
        }
        match &self.cumulative_period {
            Some(snapshot) => {
                encoder.u8(1);
                encoder.text(&snapshot.period_id)?;
                encoder.i128(snapshot.cumulative_base_atomic);
                encoder.bytes(&snapshot.evidence_digest);
            }
            None => encoder.u8(0),
        }
        encode_jurisdiction_resolution(encoder, &self.jurisdiction_resolution)?;
        encode_fx_conversion(encoder, &self.fx_conversion)
    }

    fn normalized_for_calculation(&self) -> Result<Cow<'_, Self>, TaxError> {
        self.validate()?;
        let Some(fx) = &self.fx_conversion else {
            return Ok(Cow::Borrowed(self));
        };
        let mut normalized = self.clone();
        for line in &mut normalized.lines {
            line.gross_atomic = fx.convert_amount(line.gross_atomic)?;
            line.net_atomic = fx.convert_amount(line.net_atomic)?;
        }
        if let Some(snapshot) = &mut normalized.cumulative_period {
            snapshot.cumulative_base_atomic = fx.convert_amount(snapshot.cumulative_base_atomic)?;
        }
        normalized.fx_conversion = None;
        normalized.validate()?;
        Ok(Cow::Owned(normalized))
    }
}

impl TaxFacts {
    fn validate(&self) -> Result<(), TaxError> {
        if self.atomic_decimals > 9 {
            return Err(TaxError::InvalidFacts(
                "atomic decimal scale must be between 0 and 9",
            ));
        }
        if self.gross_atomic < 0 || self.net_atomic < 0 {
            return Err(TaxError::InvalidFacts("amounts must be non-negative"));
        }
        if self.gross_atomic > MAX_ATOMIC_AMOUNT || self.net_atomic > MAX_ATOMIC_AMOUNT {
            return Err(TaxError::InvalidFacts(
                "amounts must fit Solana u64 atomic units",
            ));
        }
        if self.currency.trim().is_empty() || self.jurisdiction.trim().is_empty() {
            return Err(TaxError::InvalidFacts(
                "currency and jurisdiction must be non-empty",
            ));
        }
        if let Some(conversion) = &self.fx_conversion {
            conversion.validate_for_target(&self.currency)?;
        }
        if let Some(resolution) = &self.jurisdiction_resolution
            && (resolution.policy_id.trim().is_empty()
                || resolution.policy_version == 0
                || resolution.policy_digest == [0; 32]
                || resolution.evidence_digest == [0; 32]
                || resolution.candidate_count == 0
                || resolution.candidate_count > jurisdiction::MAX_CANDIDATES as u32
                || resolution.jurisdiction_code != self.jurisdiction
                || !resolution.trace.iter().any(|entry| entry.selected))
        {
            return Err(TaxError::InvalidFacts(
                "jurisdiction resolution evidence is invalid or does not bind to the facts",
            ));
        }
        if self.exemption_codes.len() > MAX_EXEMPTION_CODES {
            return Err(TaxError::InvalidFacts("too many exemption codes"));
        }
        for code in &self.exemption_codes {
            if code.trim().is_empty() || code.len() > MAX_CODE_BYTES {
                return Err(TaxError::InvalidFacts("invalid exemption code"));
            }
        }
        if let Some(snapshot) = &self.cumulative_period
            && (snapshot.period_id.trim().is_empty()
                || snapshot.period_id.len() > MAX_PERIOD_ID_BYTES
                || snapshot.cumulative_base_atomic < 0
                || snapshot.cumulative_base_atomic > MAX_ATOMIC_AMOUNT
                || snapshot.evidence_digest == [0; 32])
        {
            return Err(TaxError::InvalidFacts(
                "invalid cumulative period evidence snapshot",
            ));
        }
        if let Some(applicability) = &self.applicability {
            validate_applicability_binding(
                applicability,
                self.pre_applicability_digest_without_validation()?,
            )?;
        }
        Ok(())
    }

    pub fn pre_applicability_digest(&self) -> Result<Digest32, TaxError> {
        let mut without_applicability = self.clone();
        without_applicability.applicability = None;
        without_applicability.validate()?;
        without_applicability.pre_applicability_digest_without_validation()
    }

    fn pre_applicability_digest_without_validation(&self) -> Result<Digest32, TaxError> {
        let mut encoder = CanonicalEncoder::new();
        self.encode_base(&mut encoder)?;
        Ok(encoder.digest())
    }

    pub fn canonical_digest(&self) -> Result<Digest32, TaxError> {
        self.validate()?;
        let mut encoder = CanonicalEncoder::new();
        self.encode_base(&mut encoder)?;
        encode_optional_applicability(&mut encoder, &self.applicability)?;
        Ok(encoder.digest())
    }

    fn encode_base(&self, encoder: &mut CanonicalEncoder) -> Result<(), TaxError> {
        encoder.i128(self.gross_atomic);
        encoder.i128(self.net_atomic);
        encoder.u8(self.atomic_decimals);
        encoder.text(&self.currency)?;
        encoder.text(&self.jurisdiction)?;
        let mut exemption_codes = self.exemption_codes.iter().collect::<Vec<_>>();
        exemption_codes.sort();
        encoder.count(exemption_codes.len())?;
        for code in exemption_codes {
            encoder.text(code)?;
        }
        match &self.cumulative_period {
            Some(snapshot) => {
                encoder.u8(1);
                encoder.text(&snapshot.period_id)?;
                encoder.i128(snapshot.cumulative_base_atomic);
                encoder.bytes(&snapshot.evidence_digest);
            }
            None => encoder.u8(0),
        }
        // Preserve the pre-ADR-0102 canonical bytes for legacy facts without
        // a resolution envelope. Presence of an envelope is an additive
        // versioned suffix, so historical evidence remains replayable.
        encode_jurisdiction_resolution(encoder, &self.jurisdiction_resolution)?;
        encode_fx_conversion(encoder, &self.fx_conversion)
    }

    fn normalized_for_calculation(&self) -> Result<Cow<'_, Self>, TaxError> {
        self.validate()?;
        let Some(fx) = &self.fx_conversion else {
            return Ok(Cow::Borrowed(self));
        };
        let mut normalized = self.clone();
        normalized.gross_atomic = fx.convert_amount(normalized.gross_atomic)?;
        normalized.net_atomic = fx.convert_amount(normalized.net_atomic)?;
        if let Some(snapshot) = &mut normalized.cumulative_period {
            snapshot.cumulative_base_atomic = fx.convert_amount(snapshot.cumulative_base_atomic)?;
        }
        normalized.fx_conversion = None;
        normalized.validate()?;
        Ok(Cow::Owned(normalized))
    }
}

fn validate_applicability_binding(
    decision: &ApplicabilityDecision,
    base_facts_digest: Digest32,
) -> Result<(), TaxError> {
    if decision.policy_version == 0 {
        return Err(TaxError::InvalidApplicabilityDecision);
    }
    decision
        .canonical_digest()
        .map_err(|_| TaxError::InvalidApplicabilityDecision)?;
    if decision.facts_digest != base_facts_digest {
        return Err(TaxError::ApplicabilityFactsDigestMismatch);
    }
    Ok(())
}

fn applicability_treatment(
    decision: Option<&ApplicabilityDecision>,
) -> Result<Treatment, TaxError> {
    let treatment = decision.map_or(Treatment::Taxable, |decision| decision.treatment);
    if treatment == Treatment::Unresolved {
        return Err(TaxError::UnresolvedApplicability);
    }
    Ok(treatment)
}

fn treatment_uses_rule_graph(treatment: Treatment) -> bool {
    matches!(
        treatment,
        Treatment::Taxable | Treatment::ReverseCharge | Treatment::Withholding
    )
}

fn treatment_outcome(
    treatment: Treatment,
    gross_atomic: AtomicAmount,
    arithmetic_tax_atomic: AtomicAmount,
) -> Result<TreatmentOutcome, TaxError> {
    if !(0..=MAX_ATOMIC_AMOUNT).contains(&gross_atomic)
        || !(0..=MAX_ATOMIC_AMOUNT).contains(&arithmetic_tax_atomic)
    {
        return Err(TaxError::InvalidFacts(
            "treatment outcome amounts must fit Solana atomic units",
        ));
    }
    let (collected_tax_atomic, self_assessed_tax_atomic, withheld_tax_atomic, taxable_base_atomic) =
        match treatment {
            Treatment::Taxable => (arithmetic_tax_atomic, 0, 0, gross_atomic),
            Treatment::ZeroRated => (0, 0, 0, gross_atomic),
            Treatment::Exempt | Treatment::OutOfScope => (0, 0, 0, 0),
            Treatment::ReverseCharge => (0, arithmetic_tax_atomic, 0, gross_atomic),
            Treatment::Withholding => (
                arithmetic_tax_atomic,
                0,
                arithmetic_tax_atomic,
                gross_atomic,
            ),
            Treatment::Unresolved => return Err(TaxError::UnresolvedApplicability),
        };
    Ok(TreatmentOutcome {
        treatment,
        arithmetic_tax_atomic,
        collected_tax_atomic,
        self_assessed_tax_atomic,
        withheld_tax_atomic,
        taxable_base_atomic,
    })
}

fn apply_calculation_treatment(
    mut result: CalculationResult,
    treatment: Treatment,
) -> Result<CalculationResult, TaxError> {
    let arithmetic_tax_atomic = result.total_tax_atomic;
    let outcome = treatment_outcome(treatment, result.gross_atomic, arithmetic_tax_atomic)?;
    if treatment == Treatment::ReverseCharge {
        for component in &mut result.components {
            component.clamped_tax_atomic = 0;
            component.tax_atomic = 0;
            component.explanation = format!(
                "{} calculated as self-assessed tax and not collected from seller proceeds",
                component.label
            );
        }
    }
    if !treatment_uses_rule_graph(treatment) {
        result.components.clear();
    }
    result.total_tax_atomic = outcome.collected_tax_atomic;
    result.total_net_atomic = result
        .gross_atomic
        .checked_sub(result.total_tax_atomic)
        .ok_or(TaxError::ArithmeticOverflow)?;
    result.treatment_outcome = outcome;
    Ok(result)
}

fn apply_document_treatment(
    mut result: TaxDocumentResult,
    treatment: Treatment,
) -> Result<TaxDocumentResult, TaxError> {
    let arithmetic_tax_atomic = result.total_tax_atomic;
    let outcome = treatment_outcome(treatment, result.gross_atomic, arithmetic_tax_atomic)?;
    if treatment == Treatment::ReverseCharge {
        for line in &mut result.lines {
            for component in &mut line.components {
                component.clamped_tax_atomic = 0;
                component.tax_atomic = 0;
                component.explanation = format!(
                    "{} calculated as self-assessed tax and not collected from seller proceeds",
                    component.label
                );
            }
            line.total_tax_atomic = 0;
            line.total_net_atomic = line.gross_atomic;
        }
    }
    if !treatment_uses_rule_graph(treatment) {
        for line in &mut result.lines {
            line.components.clear();
            line.total_tax_atomic = 0;
            line.total_net_atomic = line.gross_atomic;
        }
    }
    result.total_tax_atomic = outcome.collected_tax_atomic;
    result.total_net_atomic = result
        .gross_atomic
        .checked_sub(result.total_tax_atomic)
        .ok_or(TaxError::ArithmeticOverflow)?;
    result.treatment_outcome = outcome;
    Ok(result)
}

fn empty_document_lines(facts: &TaxDocumentFacts) -> Result<Vec<TaxLineResult>, TaxError> {
    let mut lines = facts.lines.iter().collect::<Vec<_>>();
    lines.sort_by(|left, right| left.id.cmp(&right.id));
    lines
        .into_iter()
        .map(|line| {
            Ok(TaxLineResult {
                id: line.id.clone(),
                gross_atomic: line.gross_atomic,
                total_tax_atomic: 0,
                total_net_atomic: line.gross_atomic,
                components: Vec::new(),
            })
        })
        .collect()
}

fn encode_optional_applicability(
    encoder: &mut CanonicalEncoder,
    decision: &Option<ApplicabilityDecision>,
) -> Result<(), TaxError> {
    if let Some(decision) = decision {
        encoder.u8(1);
        let digest = decision
            .canonical_digest()
            .map_err(|_| TaxError::InvalidApplicabilityDecision)?;
        encoder.bytes(&digest);
    }
    Ok(())
}

fn encode_jurisdiction_resolution(
    encoder: &mut CanonicalEncoder,
    resolution: &Option<JurisdictionResolution>,
) -> Result<(), TaxError> {
    if let Some(resolution) = resolution {
        encoder.u8(1);
        encoder.text(&resolution.policy_id)?;
        encoder.u32(resolution.policy_version);
        encoder.bytes(&resolution.policy_digest);
        encoder.bytes(&resolution.evidence_digest);
        encoder.text(&resolution.jurisdiction_code)?;
        encoder.text(&resolution.source.canonical_name())?;
        encoder.u32(resolution.candidate_count);
        encoder.count(resolution.trace.len())?;
        for entry in &resolution.trace {
            encoder.text(&entry.source.canonical_name())?;
            encoder.count(entry.jurisdiction_codes.len())?;
            for code in &entry.jurisdiction_codes {
                encoder.text(code)?;
            }
            encoder.u32(entry.candidate_count);
            encoder.u8(u8::from(entry.selected));
        }
    }
    Ok(())
}

fn encode_fx_conversion(
    encoder: &mut CanonicalEncoder,
    conversion: &Option<FxConversionEvidence>,
) -> Result<(), TaxError> {
    if let Some(conversion) = conversion {
        encoder.u8(1);
        encoder.text(&conversion.source_currency)?;
        encoder.text(&conversion.target_currency)?;
        encoder.i128(conversion.numerator);
        encoder.i128(conversion.denominator);
        encoder.u8(match conversion.rounding_mode {
            RoundingMode::Floor => 0,
            RoundingMode::Ceil => 1,
            RoundingMode::HalfUp => 2,
            RoundingMode::HalfEven => 3,
            RoundingMode::AwayFromZero => 4,
        });
        encoder.i128(conversion.quantum_atomic);
        encoder.text(&conversion.provider)?;
        encoder.text(&conversion.quote_id)?;
        encoder.i128(i128::from(conversion.observed_at_unix));
        encoder.bytes(&conversion.evidence_digest);
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum BaseReference {
    Gross,
    Net,
    Component(String),
    Weighted {
        source: WeightedBaseSource,
        #[serde(with = "serde_i128_string")]
        numerator: i128,
        #[serde(with = "serde_i128_string")]
        denominator: i128,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WeightedBaseSource {
    Gross,
    Net,
    Component(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaxBase {
    pub sources: Vec<BaseReference>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_option_i128_string"
    )]
    pub floor_atomic: Option<AtomicAmount>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_option_i128_string"
    )]
    pub cap_atomic: Option<AtomicAmount>,
}

impl TaxBase {
    pub fn single(source: BaseReference) -> Self {
        Self {
            sources: vec![source],
            floor_atomic: None,
            cap_atomic: None,
        }
    }

    pub fn weighted(source: WeightedBaseSource, numerator: i128, denominator: i128) -> Self {
        Self::single(BaseReference::Weighted {
            source,
            numerator,
            denominator,
        })
    }

    pub fn with_bounds(
        mut self,
        floor_atomic: Option<AtomicAmount>,
        cap_atomic: Option<AtomicAmount>,
    ) -> Self {
        self.floor_atomic = floor_atomic;
        self.cap_atomic = cap_atomic;
        self
    }
}

impl BaseReference {
    pub fn weighted(source: WeightedBaseSource, numerator: i128, denominator: i128) -> Self {
        Self::Weighted {
            source,
            numerator,
            denominator,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ThresholdBasis {
    PerTransaction,
    CumulativePeriod,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ThresholdMode {
    ExemptUpTo,
    TriggerAt,
}

/// Explicit equations are required when a threshold is applied to an
/// inclusive price. `GrossExcess` treats the amount above the threshold as a
/// new inclusive gross base; this keeps the equation bounded and exact for
/// flat and progressive rates without inferring a jurisdiction's legal rule.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InclusiveThresholdEquation {
    GrossExcess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThresholdRule {
    #[serde(with = "serde_i128_string")]
    pub amount_atomic: AtomicAmount,
    pub basis: ThresholdBasis,
    pub mode: ThresholdMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inclusive_equation: Option<InclusiveThresholdEquation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rate {
    #[serde(with = "serde_i128_string")]
    pub numerator: i128,
    #[serde(with = "serde_i128_string")]
    pub denominator: i128,
}

impl Rate {
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, TaxError> {
        if numerator < 0 || denominator <= 0 {
            return Err(TaxError::InvalidRule(
                "rate must be non-negative with a positive denominator",
            ));
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    fn as_rational(&self) -> Result<Rational, TaxError> {
        Rational::new(self.numerator, self.denominator)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bracket {
    #[serde(with = "serde_i128_string")]
    pub lower_atomic: AtomicAmount,
    #[serde(with = "serde_option_i128_string")]
    pub upper_atomic: Option<AtomicAmount>,
    pub rate: Rate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RateSpec {
    Flat(Rate),
    Progressive(Vec<Bracket>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PriceMode {
    Exclusive,
    Inclusive,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaxComponentRule {
    pub id: String,
    pub label: String,
    pub base: TaxBase,
    pub rate: RateSpec,
    pub price_mode: PriceMode,
    #[serde(with = "serde_i128_string")]
    pub credit_atomic: AtomicAmount,
    #[serde(with = "serde_option_i128_string")]
    pub floor_atomic: Option<AtomicAmount>,
    #[serde(with = "serde_option_i128_string")]
    pub cap_atomic: Option<AtomicAmount>,
    pub rounding: RoundingSpec,
    #[serde(default)]
    pub threshold: Option<ThresholdRule>,
    #[serde(default)]
    pub exemption_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuleSet {
    pub id: String,
    pub version: u32,
    pub components: Vec<TaxComponentRule>,
}

/// A self-describing, versioned transport document for an immutable [`RuleSet`].
///
/// The envelope is deliberately outside `RuleSet`: the financial rule digest
/// remains stable when the document transport schema evolves.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuleSetDocument {
    pub schema: String,
    pub rule_set: RuleSet,
}

impl RuleSetDocument {
    pub fn new(rule_set: RuleSet) -> Self {
        let schema = if rule_set.uses_v2_base_operations() {
            RULE_SET_DOCUMENT_SCHEMA_V2
        } else {
            RULE_SET_DOCUMENT_SCHEMA_V1
        };
        Self {
            schema: schema.to_string(),
            rule_set,
        }
    }

    pub fn validate(&self) -> Result<(), TaxError> {
        if self.schema != RULE_SET_DOCUMENT_SCHEMA_V1 && self.schema != RULE_SET_DOCUMENT_SCHEMA_V2
        {
            return Err(TaxError::InvalidRuleDocumentSchema);
        }
        if self.schema == RULE_SET_DOCUMENT_SCHEMA_V1 && self.rule_set.uses_v2_base_operations() {
            return Err(TaxError::InvalidRuleDocumentSchema);
        }
        self.rule_set.compile().map(|_| ())
    }

    pub fn rule_digest(&self) -> Result<Digest32, TaxError> {
        self.validate()?;
        Ok(self.rule_set.compile()?.rule_digest())
    }
}

impl RuleSet {
    fn uses_v2_base_operations(&self) -> bool {
        self.components.iter().any(|component| {
            component.base.floor_atomic.is_some()
                || component.base.cap_atomic.is_some()
                || component
                    .base
                    .sources
                    .iter()
                    .any(|source| matches!(source, BaseReference::Weighted { .. }))
        })
    }

    pub fn canonical_digest(&self) -> Result<Digest32, TaxError> {
        validate_rule_set(self)?;
        let mut encoder = CanonicalEncoder::new();
        encoder.text(&self.id)?;
        encoder.u32(self.version);
        let mut components = self.components.iter().collect::<Vec<_>>();
        components.sort_by(|left, right| left.id.cmp(&right.id));
        encoder.count(components.len())?;
        for component in components {
            encode_component(&mut encoder, component)?;
        }
        Ok(encoder.digest())
    }

    /// Compile an approved immutable rule set for repeated low-latency use.
    ///
    /// Compilation performs validation, canonical digesting, and dependency
    /// ordering once. The resulting value owns its rule set and exposes no
    /// mutable access, so callers can safely share it across request tasks.
    pub fn compile(&self) -> Result<CompiledRuleSet, TaxError> {
        validate_rule_set(self)?;
        let indices: HashMap<&str, usize> = self
            .components
            .iter()
            .enumerate()
            .map(|(index, component)| (component.id.as_str(), index))
            .collect();
        let mut state = vec![VisitState::Unvisited; self.components.len()];
        let mut evaluation_order = Vec::with_capacity(self.components.len());
        for index in 0..self.components.len() {
            visit(index, self, &indices, &mut state, &mut evaluation_order)?;
        }
        Ok(CompiledRuleSet {
            rule_set: self.clone(),
            evaluation_order,
            rule_digest: self.canonical_digest()?,
        })
    }
}

/// An immutable, validated tax rule graph suitable for process-local caching.
///
/// This is intentionally separate from the persisted [`RuleSet`]. A compiled
/// value is an optimization artifact, never an independent source of truth;
/// its digest always comes from the canonical persisted rule representation.
#[derive(Clone, Debug)]
pub struct CompiledRuleSet {
    rule_set: RuleSet,
    evaluation_order: Vec<usize>,
    rule_digest: Digest32,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AllocationKey {
    line_id: String,
    component_id: String,
}

#[derive(Clone, Debug)]
struct PendingComponent {
    key: AllocationKey,
    result: ComponentResult,
    exact: Rational,
}

impl CompiledRuleSet {
    pub fn rule_set(&self) -> &RuleSet {
        &self.rule_set
    }

    pub fn rule_digest(&self) -> Digest32 {
        self.rule_digest
    }

    pub fn calculate(&self, facts: &TaxFacts) -> Result<CalculationResult, TaxError> {
        facts.validate()?;
        let treatment = applicability_treatment(facts.applicability.as_ref())?;
        let facts_digest = facts.canonical_digest()?;
        let fx_conversion = facts.fx_conversion.clone();
        let facts = facts.normalized_for_calculation()?;
        if !treatment_uses_rule_graph(treatment) {
            return Ok(CalculationResult {
                rule_set_id: self.rule_set.id.clone(),
                rule_version: self.rule_set.version,
                engine_version: ENGINE_VERSION.to_string(),
                atomic_decimals: facts.atomic_decimals,
                currency: facts.currency.clone(),
                jurisdiction: facts.jurisdiction.clone(),
                jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
                fx_conversion,
                applicability: facts.applicability.clone(),
                facts_digest,
                rule_digest: self.rule_digest,
                gross_atomic: facts.gross_atomic,
                total_tax_atomic: 0,
                total_net_atomic: facts.gross_atomic,
                treatment_outcome: treatment_outcome(treatment, facts.gross_atomic, 0)?,
                components: Vec::new(),
            });
        }
        match self.rule_set.components[0].rounding.scope {
            RoundingScope::Settlement => {
                let result =
                    self.calculate_settlement_scope(facts.as_ref(), facts_digest, fx_conversion)?;
                return apply_calculation_treatment(result, treatment);
            }
            RoundingScope::Line | RoundingScope::Invoice => {
                return Err(TaxError::UnsupportedDocumentScope(
                    "line and invoice rules require calculate_document",
                ));
            }
            RoundingScope::Component => {}
        }
        let mut computed = BTreeMap::<String, AtomicAmount>::new();
        let mut results = BTreeMap::<String, ComponentResult>::new();
        for &index in &self.evaluation_order {
            let component = &self.rule_set.components[index];
            let base_atomic = evaluate_base(&component.base, facts.as_ref(), &computed)?;
            let exemption_applied = component
                .exemption_codes
                .iter()
                .any(|code| facts.exemption_codes.iter().any(|fact| fact == code));
            let effective_base_atomic = if exemption_applied {
                0
            } else {
                apply_threshold(component, base_atomic, facts.as_ref())?
            };
            let raw_tax = if exemption_applied {
                Rational::zero()
            } else {
                calculate_raw_tax(component, effective_base_atomic)?
            };
            let after_credit =
                raw_tax.checked_sub(Rational::from_integer(component.credit_atomic))?;
            let non_negative = if after_credit.is_negative() {
                Rational::zero()
            } else {
                after_credit
            };
            let floor = component.floor_atomic.unwrap_or(0);
            let with_floor = if non_negative.is_less_than(Rational::from_integer(floor))? {
                Rational::from_integer(floor)
            } else {
                non_negative
            };
            let with_cap = match component.cap_atomic {
                Some(cap) if with_floor.is_greater_than(Rational::from_integer(cap))? => {
                    Rational::from_integer(cap)
                }
                _ => with_floor,
            };
            let tax_atomic = with_cap
                .round_to_quantum(component.rounding.mode, component.rounding.quantum_atomic)?;
            if tax_atomic < 0 {
                return Err(TaxError::ArithmeticOverflow);
            }
            let result = ComponentResult {
                id: component.id.clone(),
                label: component.label.clone(),
                base_atomic,
                effective_base_atomic,
                threshold_atomic: component
                    .threshold
                    .as_ref()
                    .map(|threshold| threshold.amount_atomic),
                exemption_applied,
                raw_tax: raw_tax.parts(),
                credit_atomic: component.credit_atomic,
                clamped_tax_atomic: with_cap
                    .round_to_quantum(RoundingMode::Floor, component.rounding.quantum_atomic)?,
                tax_atomic,
                explanation: format!(
                    "{} calculated from {} atomic units using the versioned rule graph",
                    component.label, base_atomic
                ),
            };
            computed.insert(component.id.clone(), tax_atomic);
            results.insert(component.id.clone(), result);
        }

        let total_tax_atomic = results.values().try_fold(0_i128, |total, component| {
            total
                .checked_add(component.tax_atomic)
                .ok_or(TaxError::ArithmeticOverflow)
        })?;
        let total_net_atomic = facts
            .gross_atomic
            .checked_sub(total_tax_atomic)
            .ok_or(TaxError::ArithmeticOverflow)?;
        if total_net_atomic < 0 {
            return Err(TaxError::InvalidFacts(
                "total tax cannot exceed gross amount",
            ));
        }

        let result = CalculationResult {
            rule_set_id: self.rule_set.id.clone(),
            rule_version: self.rule_set.version,
            engine_version: ENGINE_VERSION.to_string(),
            atomic_decimals: facts.atomic_decimals,
            currency: facts.currency.clone(),
            jurisdiction: facts.jurisdiction.clone(),
            jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
            fx_conversion,
            applicability: facts.applicability.clone(),
            facts_digest,
            rule_digest: self.rule_digest,
            gross_atomic: facts.gross_atomic,
            total_tax_atomic,
            total_net_atomic,
            treatment_outcome: treatment_outcome(
                Treatment::Taxable,
                facts.gross_atomic,
                total_tax_atomic,
            )?,
            components: results.into_values().collect(),
        };
        apply_calculation_treatment(result, treatment)
    }

    fn calculate_settlement_scope(
        &self,
        facts: &TaxFacts,
        facts_digest: Digest32,
        fx_conversion: Option<FxConversionEvidence>,
    ) -> Result<CalculationResult, TaxError> {
        let rounding_mode = self.rule_set.components[0].rounding.mode;
        let rounding_quantum = self.rule_set.components[0].rounding.quantum_atomic;
        let pending = self.exact_components(facts)?;
        let allocated = allocate_pending(pending, rounding_mode, rounding_quantum)?;
        let total_tax_atomic = allocated.iter().try_fold(0_i128, |total, result| {
            total
                .checked_add(result.tax_atomic)
                .ok_or(TaxError::ArithmeticOverflow)
        })?;
        let total_net_atomic = facts
            .gross_atomic
            .checked_sub(total_tax_atomic)
            .ok_or(TaxError::ArithmeticOverflow)?;
        if total_net_atomic < 0 {
            return Err(TaxError::InvalidFacts(
                "total tax cannot exceed gross amount",
            ));
        }

        Ok(CalculationResult {
            rule_set_id: self.rule_set.id.clone(),
            rule_version: self.rule_set.version,
            engine_version: ENGINE_VERSION.to_string(),
            atomic_decimals: facts.atomic_decimals,
            currency: facts.currency.clone(),
            jurisdiction: facts.jurisdiction.clone(),
            jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
            fx_conversion,
            applicability: facts.applicability.clone(),
            facts_digest,
            rule_digest: self.rule_digest,
            gross_atomic: facts.gross_atomic,
            total_tax_atomic,
            total_net_atomic,
            treatment_outcome: treatment_outcome(
                Treatment::Taxable,
                facts.gross_atomic,
                total_tax_atomic,
            )?,
            components: allocated,
        })
    }

    fn exact_components(&self, facts: &TaxFacts) -> Result<Vec<PendingComponent>, TaxError> {
        let rounding_quantum = self.rule_set.components[0].rounding.quantum_atomic;
        let computed = BTreeMap::new();
        let mut pending = Vec::with_capacity(self.rule_set.components.len());
        for &index in &self.evaluation_order {
            let component = &self.rule_set.components[index];
            let base_atomic = evaluate_base(&component.base, facts, &computed)?;
            let exemption_applied = component
                .exemption_codes
                .iter()
                .any(|code| facts.exemption_codes.iter().any(|fact| fact == code));
            let effective_base_atomic = if exemption_applied {
                0
            } else {
                apply_threshold(component, base_atomic, facts)?
            };
            let raw_tax = if exemption_applied {
                Rational::zero()
            } else {
                calculate_raw_tax(component, effective_base_atomic)?
            };
            let after_credit =
                raw_tax.checked_sub(Rational::from_integer(component.credit_atomic))?;
            let non_negative = if after_credit.is_negative() {
                Rational::zero()
            } else {
                after_credit
            };
            let floor = component.floor_atomic.unwrap_or(0);
            let with_floor = if non_negative.is_less_than(Rational::from_integer(floor))? {
                Rational::from_integer(floor)
            } else {
                non_negative
            };
            let with_cap = match component.cap_atomic {
                Some(cap) if with_floor.is_greater_than(Rational::from_integer(cap))? => {
                    Rational::from_integer(cap)
                }
                _ => with_floor,
            };
            let clamped_tax_atomic =
                with_cap.round_to_quantum(RoundingMode::Floor, rounding_quantum)?;
            if clamped_tax_atomic < 0 {
                return Err(TaxError::ArithmeticOverflow);
            }
            pending.push(PendingComponent {
                key: AllocationKey {
                    line_id: String::new(),
                    component_id: component.id.clone(),
                },
                result: ComponentResult {
                    id: component.id.clone(),
                    label: component.label.clone(),
                    base_atomic,
                    effective_base_atomic,
                    threshold_atomic: component
                        .threshold
                        .as_ref()
                        .map(|threshold| threshold.amount_atomic),
                    exemption_applied,
                    raw_tax: raw_tax.parts(),
                    credit_atomic: component.credit_atomic,
                    clamped_tax_atomic,
                    tax_atomic: clamped_tax_atomic,
                    explanation: format!(
                        "{} calculated from {} atomic units using document-scope rounding",
                        component.label, base_atomic
                    ),
                },
                exact: with_cap,
            });
        }
        Ok(pending)
    }

    pub fn calculate_document(
        &self,
        facts: &TaxDocumentFacts,
    ) -> Result<TaxDocumentResult, TaxError> {
        facts.validate()?;
        let treatment = applicability_treatment(facts.applicability.as_ref())?;
        let facts_digest = facts.canonical_digest()?;
        let fx_conversion = facts.fx_conversion.clone();
        let facts = facts.normalized_for_calculation()?;
        if !treatment_uses_rule_graph(treatment) {
            let gross_atomic = facts.lines.iter().try_fold(0_i128, |total, line| {
                total
                    .checked_add(line.gross_atomic)
                    .ok_or(TaxError::ArithmeticOverflow)
            })?;
            return Ok(TaxDocumentResult {
                document_id: facts.id.clone(),
                rule_set_id: self.rule_set.id.clone(),
                rule_version: self.rule_set.version,
                engine_version: ENGINE_VERSION.to_string(),
                atomic_decimals: facts.atomic_decimals,
                currency: facts.currency.clone(),
                jurisdiction: facts.jurisdiction.clone(),
                jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
                fx_conversion,
                applicability: facts.applicability.clone(),
                facts_digest,
                rule_digest: self.rule_digest,
                gross_atomic,
                total_tax_atomic: 0,
                total_net_atomic: gross_atomic,
                treatment_outcome: treatment_outcome(treatment, gross_atomic, 0)?,
                lines: empty_document_lines(facts.as_ref())?,
            });
        }
        let scope = self.rule_set.components[0].rounding.scope;
        if !matches!(scope, RoundingScope::Line | RoundingScope::Invoice) {
            return Err(TaxError::UnsupportedDocumentScope(
                "document calculation requires line or invoice rounding",
            ));
        }
        let rounding_mode = self.rule_set.components[0].rounding.mode;
        let rounding_quantum = self.rule_set.components[0].rounding.quantum_atomic;
        let mut ordered_lines = facts.lines.iter().collect::<Vec<_>>();
        ordered_lines.sort_by(|left, right| left.id.cmp(&right.id));
        let mut pending_lines = Vec::with_capacity(ordered_lines.len());
        for line in ordered_lines {
            let line_facts = TaxFacts {
                gross_atomic: line.gross_atomic,
                net_atomic: line.net_atomic,
                atomic_decimals: facts.atomic_decimals,
                currency: facts.currency.clone(),
                jurisdiction: facts.jurisdiction.clone(),
                jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
                fx_conversion: None,
                exemption_codes: line.exemption_codes.clone(),
                cumulative_period: facts.cumulative_period.clone(),
                applicability: None,
            };
            let mut pending = self.exact_components(&line_facts)?;
            for component in &mut pending {
                component.key.line_id = line.id.clone();
            }
            pending_lines.push((line, pending));
        }

        let mut line_results = Vec::with_capacity(pending_lines.len());
        if scope == RoundingScope::Line {
            for (line, pending) in pending_lines {
                let components = allocate_pending(pending, rounding_mode, rounding_quantum)?;
                line_results.push(build_line_result(
                    line.id.clone(),
                    line.gross_atomic,
                    components,
                )?);
            }
        } else {
            let all_pending = pending_lines
                .iter()
                .flat_map(|(_, pending)| pending.iter().cloned())
                .collect::<Vec<_>>();
            let allocated = allocate_pending(all_pending, rounding_mode, rounding_quantum)?;
            let mut components_by_line = BTreeMap::<String, Vec<ComponentResult>>::new();
            let mut keys = pending_lines
                .iter()
                .flat_map(|(_, pending)| pending.iter().map(|item| item.key.clone()))
                .collect::<Vec<_>>();
            keys.sort();
            for (key, component) in keys.into_iter().zip(allocated) {
                components_by_line
                    .entry(key.line_id)
                    .or_default()
                    .push(component);
            }
            for (line, _) in pending_lines {
                let components = components_by_line
                    .remove(&line.id)
                    .ok_or(TaxError::ArithmeticOverflow)?;
                line_results.push(build_line_result(
                    line.id.clone(),
                    line.gross_atomic,
                    components,
                )?);
            }
        }

        let gross_atomic = line_results.iter().try_fold(0_i128, |total, line| {
            total
                .checked_add(line.gross_atomic)
                .ok_or(TaxError::ArithmeticOverflow)
        })?;
        let total_tax_atomic = line_results.iter().try_fold(0_i128, |total, line| {
            total
                .checked_add(line.total_tax_atomic)
                .ok_or(TaxError::ArithmeticOverflow)
        })?;
        let total_net_atomic = gross_atomic
            .checked_sub(total_tax_atomic)
            .ok_or(TaxError::ArithmeticOverflow)?;
        if total_net_atomic < 0 {
            return Err(TaxError::InvalidFacts(
                "document tax cannot exceed gross amount",
            ));
        }
        let result = TaxDocumentResult {
            document_id: facts.id.clone(),
            rule_set_id: self.rule_set.id.clone(),
            rule_version: self.rule_set.version,
            engine_version: ENGINE_VERSION.to_string(),
            atomic_decimals: facts.atomic_decimals,
            currency: facts.currency.clone(),
            jurisdiction: facts.jurisdiction.clone(),
            jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
            fx_conversion,
            applicability: facts.applicability.clone(),
            facts_digest,
            rule_digest: self.rule_digest,
            gross_atomic,
            total_tax_atomic,
            total_net_atomic,
            treatment_outcome: treatment_outcome(
                Treatment::Taxable,
                gross_atomic,
                total_tax_atomic,
            )?,
            lines: line_results,
        };
        apply_document_treatment(result, treatment)
    }

    /// Resolves and binds jurisdiction evidence while reusing this compiled
    /// immutable rule graph in the low-latency quote path.
    pub fn calculate_with_jurisdiction_resolution(
        &self,
        facts: &TaxFacts,
        policy: &jurisdiction::ResolutionPolicy,
        candidates: &[jurisdiction::JurisdictionCandidate],
    ) -> Result<CalculationResult, TaxError> {
        let resolution = policy
            .resolve(candidates)
            .map_err(TaxError::JurisdictionResolution)?;
        let mut resolved_facts = facts.clone();
        resolved_facts.jurisdiction = resolution.jurisdiction_code.clone();
        resolved_facts.jurisdiction_resolution = Some(resolution);
        self.calculate(&resolved_facts)
    }

    /// Resolves and binds jurisdiction evidence for a document calculation
    /// while reusing this compiled immutable rule graph.
    pub fn calculate_document_with_jurisdiction_resolution(
        &self,
        facts: &TaxDocumentFacts,
        policy: &jurisdiction::ResolutionPolicy,
        candidates: &[jurisdiction::JurisdictionCandidate],
    ) -> Result<TaxDocumentResult, TaxError> {
        let resolution = policy
            .resolve(candidates)
            .map_err(TaxError::JurisdictionResolution)?;
        let mut resolved_facts = facts.clone();
        resolved_facts.jurisdiction = resolution.jurisdiction_code.clone();
        resolved_facts.jurisdiction_resolution = Some(resolution);
        self.calculate_document(&resolved_facts)
    }
}

fn allocate_pending(
    mut pending: Vec<PendingComponent>,
    rounding_mode: RoundingMode,
    rounding_quantum: AtomicAmount,
) -> Result<Vec<ComponentResult>, TaxError> {
    pending.sort_by(|left, right| left.key.cmp(&right.key));
    let exact_total = pending.iter().try_fold(Rational::zero(), |total, item| {
        total.checked_add(item.exact)
    })?;
    let target_total = exact_total.round_to_quantum(rounding_mode, rounding_quantum)?;
    let floored_total = pending.iter().try_fold(0_i128, |total, item| {
        total
            .checked_add(item.result.clamped_tax_atomic)
            .ok_or(TaxError::ArithmeticOverflow)
    })?;
    let residual_atomic = target_total
        .checked_sub(floored_total)
        .ok_or(TaxError::ArithmeticOverflow)?;
    if residual_atomic < 0 || residual_atomic % rounding_quantum != 0 {
        return Err(TaxError::ArithmeticOverflow);
    }
    let mut residual_units = residual_atomic / rounding_quantum;
    let max_residual_units =
        i128::try_from(pending.len()).map_err(|_| TaxError::ArithmeticOverflow)?;
    if residual_units > max_residual_units {
        return Err(TaxError::ArithmeticOverflow);
    }

    let mut allocated = vec![false; pending.len()];
    while residual_units > 0 {
        let mut selected: Option<usize> = None;
        for index in 0..pending.len() {
            if allocated[index] {
                continue;
            }
            let candidate_remainder = pending[index].exact.checked_sub(Rational::from_integer(
                pending[index].result.clamped_tax_atomic,
            ))?;
            let should_select = match selected {
                None => true,
                Some(current) => {
                    let current_remainder = pending[current].exact.checked_sub(
                        Rational::from_integer(pending[current].result.clamped_tax_atomic),
                    )?;
                    candidate_remainder.is_greater_than(current_remainder)?
                        || (!candidate_remainder.is_greater_than(current_remainder)?
                            && !current_remainder.is_greater_than(candidate_remainder)?
                            && pending[index].key < pending[current].key)
                }
            };
            if should_select {
                selected = Some(index);
            }
        }
        let Some(index) = selected else {
            return Err(TaxError::ArithmeticOverflow);
        };
        pending[index].result.tax_atomic = pending[index]
            .result
            .tax_atomic
            .checked_add(rounding_quantum)
            .ok_or(TaxError::ArithmeticOverflow)?;
        allocated[index] = true;
        residual_units -= 1;
    }

    Ok(pending.into_iter().map(|item| item.result).collect())
}

fn build_line_result(
    id: String,
    gross_atomic: AtomicAmount,
    mut components: Vec<ComponentResult>,
) -> Result<TaxLineResult, TaxError> {
    components.sort_by(|left, right| left.id.cmp(&right.id));
    let total_tax_atomic = components.iter().try_fold(0_i128, |total, component| {
        total
            .checked_add(component.tax_atomic)
            .ok_or(TaxError::ArithmeticOverflow)
    })?;
    let total_net_atomic = gross_atomic
        .checked_sub(total_tax_atomic)
        .ok_or(TaxError::ArithmeticOverflow)?;
    if total_tax_atomic < 0 || total_net_atomic < 0 {
        return Err(TaxError::InvalidFacts(
            "line tax cannot exceed gross amount",
        ));
    }
    Ok(TaxLineResult {
        id,
        gross_atomic,
        total_tax_atomic,
        total_net_atomic,
        components,
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentResult {
    pub id: String,
    pub label: String,
    #[serde(with = "serde_i128_string")]
    pub base_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub effective_base_atomic: AtomicAmount,
    #[serde(with = "serde_option_i128_string")]
    pub threshold_atomic: Option<AtomicAmount>,
    pub exemption_applied: bool,
    pub raw_tax: RationalParts,
    #[serde(with = "serde_i128_string")]
    pub credit_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub clamped_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub tax_atomic: AtomicAmount,
    pub explanation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TreatmentOutcome {
    pub treatment: Treatment,
    #[serde(with = "serde_i128_string")]
    pub arithmetic_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub collected_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub self_assessed_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub withheld_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub taxable_base_atomic: AtomicAmount,
}

impl Default for TreatmentOutcome {
    fn default() -> Self {
        Self {
            treatment: Treatment::Taxable,
            arithmetic_tax_atomic: 0,
            collected_tax_atomic: 0,
            self_assessed_tax_atomic: 0,
            withheld_tax_atomic: 0,
            taxable_base_atomic: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CalculationResult {
    pub rule_set_id: String,
    pub rule_version: u32,
    // Missing versions are legacy evidence and must fail replay closed. New
    // calculations always populate this field explicitly below.
    #[serde(default)]
    pub engine_version: String,
    pub atomic_decimals: u8,
    pub currency: String,
    pub jurisdiction: String,
    #[serde(default)]
    pub jurisdiction_resolution: Option<JurisdictionResolution>,
    #[serde(default)]
    pub fx_conversion: Option<FxConversionEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<ApplicabilityDecision>,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_net_atomic: AtomicAmount,
    #[serde(default)]
    pub treatment_outcome: TreatmentOutcome,
    pub components: Vec<ComponentResult>,
}

impl CalculationResult {
    /// Recalculate and compare the complete immutable evidence result.
    pub fn verify_replay(&self, rule_set: &RuleSet, facts: &TaxFacts) -> Result<(), TaxError> {
        if self.engine_version != ENGINE_VERSION {
            return Err(TaxError::EngineVersionMismatch {
                expected: ENGINE_VERSION.to_string(),
                actual: self.engine_version.clone(),
            });
        }
        let replayed = calculate(rule_set, facts)?;
        if replayed != *self {
            return Err(TaxError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaxLineResult {
    pub id: String,
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_net_atomic: AtomicAmount,
    pub components: Vec<ComponentResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaxDocumentResult {
    pub document_id: String,
    pub rule_set_id: String,
    pub rule_version: u32,
    #[serde(default)]
    pub engine_version: String,
    pub atomic_decimals: u8,
    pub currency: String,
    pub jurisdiction: String,
    #[serde(default)]
    pub jurisdiction_resolution: Option<JurisdictionResolution>,
    #[serde(default)]
    pub fx_conversion: Option<FxConversionEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applicability: Option<ApplicabilityDecision>,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_net_atomic: AtomicAmount,
    #[serde(default)]
    pub treatment_outcome: TreatmentOutcome,
    pub lines: Vec<TaxLineResult>,
}

impl TaxDocumentResult {
    pub fn verify_replay(
        &self,
        rule_set: &RuleSet,
        facts: &TaxDocumentFacts,
    ) -> Result<(), TaxError> {
        if self.engine_version != ENGINE_VERSION {
            return Err(TaxError::EngineVersionMismatch {
                expected: ENGINE_VERSION.to_string(),
                actual: self.engine_version.clone(),
            });
        }
        let replayed = calculate_document(rule_set, facts)?;
        if replayed != *self {
            return Err(TaxError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum TaxError {
    #[error("invalid rule: {0}")]
    InvalidRule(&'static str),
    #[error("unsupported tax rule document schema")]
    InvalidRuleDocumentSchema,
    #[error("invalid facts: {0}")]
    InvalidFacts(&'static str),
    #[error("invalid tax applicability decision")]
    InvalidApplicabilityDecision,
    #[error("tax applicability decision is bound to a different facts digest")]
    ApplicabilityFactsDigestMismatch,
    #[error("tax applicability is unresolved")]
    UnresolvedApplicability,
    #[error("tax applicability treatment is not yet integrated: {0:?}")]
    UnsupportedApplicabilityTreatment(Treatment),
    #[error("missing tax component: {0}")]
    MissingComponent(String),
    #[error("tax rule dependency cycle detected at component: {0}")]
    DependencyCycle(String),
    #[error("arithmetic overflow")]
    ArithmeticOverflow,
    #[error("tax base is not an integral atomic amount")]
    NonIntegralTaxBase,
    #[error("unsupported inclusive calculation: {0}")]
    UnsupportedInclusive(&'static str),
    #[error("cumulative threshold requires an evidence snapshot")]
    MissingCumulativeSnapshot,
    #[error("unsupported threshold calculation: {0}")]
    UnsupportedThreshold(&'static str),
    #[error("unsupported document calculation scope: {0}")]
    UnsupportedDocumentScope(&'static str),
    #[error("calculation result was produced by engine version {actual}, expected {expected}")]
    EngineVersionMismatch { expected: String, actual: String },
    #[error("calculation replay does not match the stored result")]
    ReplayMismatch,
    #[error("jurisdiction resolution failed: {0}")]
    JurisdictionResolution(#[from] jurisdiction::JurisdictionError),
    #[error("FX conversion result exceeds the supported atomic amount bound")]
    FxConversionOutOfRange,
}

pub fn calculate(rule_set: &RuleSet, facts: &TaxFacts) -> Result<CalculationResult, TaxError> {
    rule_set.compile()?.calculate(facts)
}

pub fn calculate_document(
    rule_set: &RuleSet,
    facts: &TaxDocumentFacts,
) -> Result<TaxDocumentResult, TaxError> {
    rule_set.compile()?.calculate_document(facts)
}

/// Resolve and bind jurisdiction evidence before calculating.
///
/// Application adapters should use this boundary after authenticating and
/// normalizing provider evidence. It prevents a caller-provided jurisdiction
/// string or digest from being the authority for a new calculation while
/// keeping the arithmetic engine pure and replayable.
pub fn calculate_with_jurisdiction_resolution(
    rule_set: &RuleSet,
    facts: &TaxFacts,
    policy: &jurisdiction::ResolutionPolicy,
    candidates: &[jurisdiction::JurisdictionCandidate],
) -> Result<CalculationResult, TaxError> {
    let resolution = policy
        .resolve(candidates)
        .map_err(TaxError::JurisdictionResolution)?;
    let mut resolved_facts = facts.clone();
    resolved_facts.jurisdiction = resolution.jurisdiction_code.clone();
    resolved_facts.jurisdiction_resolution = Some(resolution);
    calculate(rule_set, &resolved_facts)
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i128(&mut self, value: i128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn text(&mut self, value: &str) -> Result<(), TaxError> {
        self.count(value.len())?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn count(&mut self, value: usize) -> Result<(), TaxError> {
        self.u32(u32::try_from(value).map_err(|_| TaxError::ArithmeticOverflow)?);
        Ok(())
    }

    fn optional_i128(&mut self, value: Option<i128>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.i128(value);
            }
            None => self.u8(0),
        }
    }

    fn digest(self) -> Digest32 {
        Sha256::digest(self.bytes).into()
    }
}

fn encode_component(
    encoder: &mut CanonicalEncoder,
    component: &TaxComponentRule,
) -> Result<(), TaxError> {
    encoder.text(&component.id)?;
    encoder.text(&component.label)?;
    encoder.count(component.base.sources.len())?;
    for source in &component.base.sources {
        match source {
            BaseReference::Gross => encoder.u8(0),
            BaseReference::Net => encoder.u8(1),
            BaseReference::Component(id) => {
                encoder.u8(2);
                encoder.text(id)?;
            }
            BaseReference::Weighted {
                source,
                numerator,
                denominator,
            } => {
                encoder.u8(3);
                encode_weighted_base_source(encoder, source)?;
                encoder.i128(*numerator);
                encoder.i128(*denominator);
            }
        }
    }
    if component.base.floor_atomic.is_some() || component.base.cap_atomic.is_some() {
        encoder.u8(1);
        encoder.optional_i128(component.base.floor_atomic);
        encoder.optional_i128(component.base.cap_atomic);
    }
    match &component.rate {
        RateSpec::Flat(rate) => {
            encoder.u8(0);
            encode_rate(encoder, rate);
        }
        RateSpec::Progressive(brackets) => {
            encoder.u8(1);
            encoder.count(brackets.len())?;
            for bracket in brackets {
                encoder.i128(bracket.lower_atomic);
                encoder.optional_i128(bracket.upper_atomic);
                encode_rate(encoder, &bracket.rate);
            }
        }
    }
    encoder.u8(match component.price_mode {
        PriceMode::Exclusive => 0,
        PriceMode::Inclusive => 1,
    });
    encoder.i128(component.credit_atomic);
    encoder.optional_i128(component.floor_atomic);
    encoder.optional_i128(component.cap_atomic);
    encoder.u8(match component.rounding.mode {
        RoundingMode::Floor => 0,
        RoundingMode::Ceil => 1,
        RoundingMode::HalfUp => 2,
        RoundingMode::HalfEven => 3,
        RoundingMode::AwayFromZero => 4,
    });
    // Preserve the legacy canonical bytes for the default component scope;
    // non-component scopes are additive digest markers.
    let scope_marker = match component.rounding.scope {
        RoundingScope::Component => None,
        RoundingScope::Settlement => Some(1),
        RoundingScope::Line => Some(2),
        RoundingScope::Invoice => Some(3),
    };
    if let Some(scope_marker) = scope_marker {
        encoder.u8(scope_marker);
    }
    // Preserve legacy bytes for the default atomic quantum; a non-default
    // quantum is an explicit semantic change to the rule digest.
    if component.rounding.quantum_atomic != 1 {
        encoder.u8(2);
        encoder.i128(component.rounding.quantum_atomic);
    }
    match &component.threshold {
        Some(threshold) => {
            encoder.u8(1);
            encoder.i128(threshold.amount_atomic);
            encoder.u8(match threshold.basis {
                ThresholdBasis::PerTransaction => 0,
                ThresholdBasis::CumulativePeriod => 1,
            });
            encoder.u8(match threshold.mode {
                ThresholdMode::ExemptUpTo => 0,
                ThresholdMode::TriggerAt => 1,
            });
            // Preserve legacy rule digests when no inclusive equation is
            // declared. An explicit equation is an additive semantic marker.
            if let Some(equation) = threshold.inclusive_equation {
                encoder.u8(2);
                encoder.u8(match equation {
                    InclusiveThresholdEquation::GrossExcess => 0,
                });
            }
        }
        None => encoder.u8(0),
    }
    let mut exemption_codes = component.exemption_codes.iter().collect::<Vec<_>>();
    exemption_codes.sort();
    encoder.count(exemption_codes.len())?;
    for code in exemption_codes {
        encoder.text(code)?;
    }
    Ok(())
}

fn encode_weighted_base_source(
    encoder: &mut CanonicalEncoder,
    source: &WeightedBaseSource,
) -> Result<(), TaxError> {
    match source {
        WeightedBaseSource::Gross => encoder.u8(0),
        WeightedBaseSource::Net => encoder.u8(1),
        WeightedBaseSource::Component(id) => {
            encoder.u8(2);
            encoder.text(id)?;
        }
    }
    Ok(())
}

fn encode_rate(encoder: &mut CanonicalEncoder, rate: &Rate) {
    encoder.i128(rate.numerator);
    encoder.i128(rate.denominator);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn visit(
    index: usize,
    rule_set: &RuleSet,
    indices: &HashMap<&str, usize>,
    state: &mut [VisitState],
    order: &mut Vec<usize>,
) -> Result<(), TaxError> {
    match state[index] {
        VisitState::Visited => return Ok(()),
        VisitState::Visiting => {
            return Err(TaxError::DependencyCycle(
                rule_set.components[index].id.clone(),
            ));
        }
        VisitState::Unvisited => {}
    }
    state[index] = VisitState::Visiting;
    for dependency in &rule_set.components[index].base.sources {
        if let Some(id) = component_dependency(dependency) {
            let dependency_index = indices
                .get(id)
                .copied()
                .ok_or_else(|| TaxError::MissingComponent(id.to_string()))?;
            visit(dependency_index, rule_set, indices, state, order)?;
        }
    }
    state[index] = VisitState::Visited;
    order.push(index);
    Ok(())
}

fn evaluate_base(
    base: &TaxBase,
    facts: &TaxFacts,
    computed: &BTreeMap<String, AtomicAmount>,
) -> Result<AtomicAmount, TaxError> {
    if base.sources.is_empty() {
        return Err(TaxError::InvalidRule(
            "tax base must have at least one source",
        ));
    }
    if is_legacy_additive_base(base) {
        return base.sources.iter().try_fold(0_i128, |total, source| {
            let value = match source {
                BaseReference::Gross => facts.gross_atomic,
                BaseReference::Net => facts.net_atomic,
                BaseReference::Component(id) => computed
                    .get(id)
                    .copied()
                    .ok_or_else(|| TaxError::MissingComponent(id.clone()))?,
                BaseReference::Weighted { .. } => {
                    return Err(TaxError::InvalidRule(
                        "weighted source requires generalized base evaluation",
                    ));
                }
            };
            total.checked_add(value).ok_or(TaxError::ArithmeticOverflow)
        });
    }
    let total = base
        .sources
        .iter()
        .try_fold(Rational::zero(), |total, source| {
            total.checked_add(evaluate_base_term(source, facts, computed)?)
        })?;
    if total.numerator % total.denominator != 0 {
        return Err(TaxError::NonIntegralTaxBase);
    }
    let integral = total.numerator / total.denominator;
    let floored = match base.floor_atomic {
        Some(floor) => integral.max(floor),
        None => integral,
    };
    Ok(match base.cap_atomic {
        Some(cap) => floored.min(cap),
        None => floored,
    })
}

fn is_legacy_additive_base(base: &TaxBase) -> bool {
    base.floor_atomic.is_none()
        && base.cap_atomic.is_none()
        && base.sources.iter().all(|source| {
            matches!(
                source,
                BaseReference::Gross | BaseReference::Net | BaseReference::Component(_)
            )
        })
}

fn evaluate_base_term(
    source: &BaseReference,
    facts: &TaxFacts,
    computed: &BTreeMap<String, AtomicAmount>,
) -> Result<Rational, TaxError> {
    let (value, coefficient) = match source {
        BaseReference::Gross => (facts.gross_atomic, Rational::from_integer(1)),
        BaseReference::Net => (facts.net_atomic, Rational::from_integer(1)),
        BaseReference::Component(id) => (
            computed
                .get(id)
                .copied()
                .ok_or_else(|| TaxError::MissingComponent(id.clone()))?,
            Rational::from_integer(1),
        ),
        BaseReference::Weighted {
            source,
            numerator,
            denominator,
        } => {
            let value = match source {
                WeightedBaseSource::Gross => facts.gross_atomic,
                WeightedBaseSource::Net => facts.net_atomic,
                WeightedBaseSource::Component(id) => computed
                    .get(id)
                    .copied()
                    .ok_or_else(|| TaxError::MissingComponent(id.clone()))?,
            };
            (value, Rational::new(*numerator, *denominator)?)
        }
    };
    Rational::from_integer(value).checked_mul(coefficient)
}

fn component_dependency(source: &BaseReference) -> Option<&str> {
    match source {
        BaseReference::Component(id) => Some(id.as_str()),
        BaseReference::Weighted {
            source: WeightedBaseSource::Component(id),
            ..
        } => Some(id.as_str()),
        BaseReference::Gross | BaseReference::Net | BaseReference::Weighted { .. } => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BaseSourceIdentity<'a> {
    Gross,
    Net,
    Component(&'a str),
}

fn base_source_identity(source: &BaseReference) -> BaseSourceIdentity<'_> {
    match source {
        BaseReference::Gross
        | BaseReference::Weighted {
            source: WeightedBaseSource::Gross,
            ..
        } => BaseSourceIdentity::Gross,
        BaseReference::Net
        | BaseReference::Weighted {
            source: WeightedBaseSource::Net,
            ..
        } => BaseSourceIdentity::Net,
        BaseReference::Component(id)
        | BaseReference::Weighted {
            source: WeightedBaseSource::Component(id),
            ..
        } => BaseSourceIdentity::Component(id),
    }
}

fn apply_threshold(
    component: &TaxComponentRule,
    base_atomic: AtomicAmount,
    facts: &TaxFacts,
) -> Result<AtomicAmount, TaxError> {
    let Some(threshold) = &component.threshold else {
        return Ok(base_atomic);
    };
    if component.price_mode == PriceMode::Inclusive && threshold.inclusive_equation.is_none() {
        return Err(TaxError::UnsupportedThreshold(
            "inclusive threshold equations require an explicit rule version",
        ));
    }
    let threshold_atomic = threshold.amount_atomic;
    match threshold.basis {
        ThresholdBasis::PerTransaction => match threshold.mode {
            ThresholdMode::ExemptUpTo => Ok(base_atomic.saturating_sub(threshold_atomic)),
            ThresholdMode::TriggerAt => {
                if base_atomic >= threshold_atomic {
                    Ok(base_atomic)
                } else {
                    Ok(0)
                }
            }
        },
        ThresholdBasis::CumulativePeriod => {
            let Some(snapshot) = &facts.cumulative_period else {
                return Err(TaxError::MissingCumulativeSnapshot);
            };
            let cumulative_after = snapshot
                .cumulative_base_atomic
                .checked_add(base_atomic)
                .ok_or(TaxError::ArithmeticOverflow)?;
            match threshold.mode {
                ThresholdMode::ExemptUpTo => {
                    let remaining =
                        threshold_atomic.saturating_sub(snapshot.cumulative_base_atomic);
                    Ok(base_atomic.saturating_sub(remaining))
                }
                ThresholdMode::TriggerAt => {
                    if cumulative_after >= threshold_atomic {
                        Ok(base_atomic)
                    } else {
                        Ok(0)
                    }
                }
            }
        }
    }
}

fn calculate_raw_tax(
    component: &TaxComponentRule,
    base_atomic: AtomicAmount,
) -> Result<Rational, TaxError> {
    if base_atomic < 0 {
        return Err(TaxError::InvalidFacts("tax base must be non-negative"));
    }
    if component.price_mode == PriceMode::Inclusive
        && !matches!(component.base.sources.as_slice(), [BaseReference::Gross])
    {
        return Err(TaxError::UnsupportedInclusive(
            "inclusive tax requires a single Gross base",
        ));
    }
    match component.price_mode {
        PriceMode::Exclusive => tax_for_base(&component.rate, base_atomic),
        PriceMode::Inclusive => inclusive_tax_for_gross(&component.rate, base_atomic),
    }
}

fn tax_for_base(rate: &RateSpec, base_atomic: AtomicAmount) -> Result<Rational, TaxError> {
    match rate {
        RateSpec::Flat(rate) => rate.as_rational()?.checked_mul_integer(base_atomic),
        RateSpec::Progressive(brackets) => {
            validate_brackets(brackets)?;
            let mut tax = Rational::zero();
            for bracket in brackets {
                if base_atomic <= bracket.lower_atomic {
                    break;
                }
                let upper = bracket.upper_atomic.unwrap_or(base_atomic);
                let taxable = base_atomic.min(upper).saturating_sub(bracket.lower_atomic);
                if taxable <= 0 {
                    continue;
                }
                tax = tax.checked_add(bracket.rate.as_rational()?.checked_mul_integer(taxable)?)?;
            }
            Ok(tax)
        }
    }
}

fn inclusive_tax_for_gross(
    rate: &RateSpec,
    gross_atomic: AtomicAmount,
) -> Result<Rational, TaxError> {
    match rate {
        RateSpec::Flat(rate) => {
            let rate = rate.as_rational()?;
            let denominator = Rational::from_integer(1).checked_add(rate)?;
            Rational::from_integer(gross_atomic)
                .checked_mul(rate)?
                .checked_div(denominator)
        }
        RateSpec::Progressive(brackets) => {
            validate_brackets(brackets)?;
            let gross = Rational::from_integer(gross_atomic);
            let mut tax_below = Rational::zero();
            for bracket in brackets {
                let lower = Rational::from_integer(bracket.lower_atomic);
                let gross_at_lower = lower.checked_add(tax_below)?;
                let rate = bracket.rate.as_rational()?;
                let upper = match bracket.upper_atomic {
                    Some(upper) => Rational::from_integer(upper),
                    None => {
                        let net_delta = gross
                            .checked_sub(gross_at_lower)?
                            .checked_div(Rational::from_integer(1).checked_add(rate)?)?;
                        return tax_below.checked_add(net_delta.checked_mul(rate)?);
                    }
                };
                let width = upper.checked_sub(lower)?;
                let gross_at_upper = gross_at_lower.checked_add(
                    width.checked_mul(Rational::from_integer(1).checked_add(rate)?)?,
                )?;
                if gross.is_less_than(gross_at_upper)? {
                    let net_delta = gross
                        .checked_sub(gross_at_lower)?
                        .checked_div(Rational::from_integer(1).checked_add(rate)?)?;
                    return tax_below.checked_add(net_delta.checked_mul(rate)?);
                }
                tax_below = tax_below.checked_add(width.checked_mul(rate)?)?;
            }
            Ok(tax_below)
        }
    }
}

fn validate_rule_set(rule_set: &RuleSet) -> Result<(), TaxError> {
    if rule_set.id.trim().is_empty() || rule_set.version == 0 {
        return Err(TaxError::InvalidRule(
            "rule set id and version are required",
        ));
    }
    if rule_set.components.is_empty() || rule_set.components.len() > MAX_COMPONENTS {
        return Err(TaxError::InvalidRule(
            "component count is outside the supported bound",
        ));
    }
    let rounding_scope = rule_set.components[0].rounding.scope;
    let rounding_mode = rule_set.components[0].rounding.mode;
    let rounding_quantum = rule_set.components[0].rounding.quantum_atomic;
    let mut ids = HashMap::new();
    for component in &rule_set.components {
        if component.rounding.quantum_atomic <= 0
            || component.rounding.quantum_atomic > MAX_ATOMIC_AMOUNT
        {
            return Err(TaxError::InvalidRule(
                "rounding quantum must be a positive Solana u64 atomic amount",
            ));
        }
        if component.rounding.scope != rounding_scope {
            return Err(TaxError::InvalidRule(
                "all components must use the same rounding scope",
            ));
        }
        if matches!(
            rounding_scope,
            RoundingScope::Settlement | RoundingScope::Line | RoundingScope::Invoice
        ) && (component.rounding.mode != rounding_mode
            || component.rounding.quantum_atomic != rounding_quantum
            || component
                .base
                .sources
                .iter()
                .any(|source| component_dependency(source).is_some()))
        {
            return Err(TaxError::InvalidRule(
                "aggregate, line, and invoice rounding cannot use component dependencies, mixed rounding modes, or mixed atomic quanta",
            ));
        }
        if component.id.trim().is_empty()
            || component.label.trim().is_empty()
            || component.id.len() > MAX_CODE_BYTES
            || component.label.len() > MAX_CODE_BYTES
        {
            return Err(TaxError::InvalidRule("component id and label are required"));
        }
        if ids.insert(component.id.as_str(), true).is_some() {
            return Err(TaxError::InvalidRule("component ids must be unique"));
        }
        if component.credit_atomic < 0
            || component.floor_atomic.unwrap_or(0) < 0
            || component.cap_atomic.unwrap_or(0) < 0
        {
            return Err(TaxError::InvalidRule(
                "credit, floor, and cap must be non-negative",
            ));
        }
        if component.credit_atomic > MAX_ATOMIC_AMOUNT
            || component.floor_atomic.unwrap_or(0) > MAX_ATOMIC_AMOUNT
            || component.cap_atomic.unwrap_or(0) > MAX_ATOMIC_AMOUNT
        {
            return Err(TaxError::InvalidRule(
                "credit, floor, and cap must fit Solana u64 atomic units",
            ));
        }
        if component
            .floor_atomic
            .is_some_and(|floor| floor % component.rounding.quantum_atomic != 0)
            || component
                .cap_atomic
                .is_some_and(|cap| cap % component.rounding.quantum_atomic != 0)
        {
            return Err(TaxError::InvalidRule(
                "floor and cap must be exact multiples of the rounding quantum",
            ));
        }
        if let (Some(floor), Some(cap)) = (component.floor_atomic, component.cap_atomic)
            && floor > cap
        {
            return Err(TaxError::InvalidRule("floor cannot exceed cap"));
        }
        if component.base.sources.is_empty() || component.base.sources.len() > MAX_COMPONENTS {
            return Err(TaxError::InvalidRule(
                "tax base source count is outside the supported bound",
            ));
        }
        if component.base.floor_atomic.unwrap_or(0) < 0
            || component.base.cap_atomic.unwrap_or(0) < 0
        {
            return Err(TaxError::InvalidRule(
                "tax base floor and cap must be non-negative",
            ));
        }
        if component.base.floor_atomic.unwrap_or(0) > MAX_ATOMIC_AMOUNT
            || component.base.cap_atomic.unwrap_or(0) > MAX_ATOMIC_AMOUNT
        {
            return Err(TaxError::InvalidRule(
                "tax base floor and cap must fit Solana u64 atomic units",
            ));
        }
        if let (Some(floor), Some(cap)) = (component.base.floor_atomic, component.base.cap_atomic)
            && floor > cap
        {
            return Err(TaxError::InvalidRule("base floor cannot exceed cap"));
        }
        for (source_index, source) in component.base.sources.iter().enumerate() {
            let identity = base_source_identity(source);
            if component.base.sources[..source_index]
                .iter()
                .any(|previous| base_source_identity(previous) == identity)
            {
                return Err(TaxError::InvalidRule(
                    "tax base sources must be distinct; repeated sources are ambiguous",
                ));
            }
            if let BaseReference::Weighted {
                source,
                numerator,
                denominator,
            } = source
            {
                if *numerator == 0 {
                    return Err(TaxError::InvalidRule(
                        "weighted tax base coefficient must be non-zero",
                    ));
                }
                if *numerator > MAX_ATOMIC_AMOUNT || *numerator < -MAX_ATOMIC_AMOUNT {
                    return Err(TaxError::InvalidRule(
                        "weighted tax base numerator must fit the bounded coefficient domain",
                    ));
                }
                if *denominator <= 0 || *denominator > MAX_ATOMIC_AMOUNT {
                    return Err(TaxError::InvalidRule(
                        "weighted tax base denominator must be positive and bounded",
                    ));
                }
                if let WeightedBaseSource::Component(id) = source
                    && (id.trim().is_empty() || id.len() > MAX_CODE_BYTES)
                {
                    return Err(TaxError::InvalidRule(
                        "weighted component source id is invalid",
                    ));
                }
            }
        }
        if component.exemption_codes.len() > MAX_EXEMPTION_CODES {
            return Err(TaxError::InvalidRule("too many exemption codes"));
        }
        let mut exemption_codes = HashMap::new();
        for code in &component.exemption_codes {
            if code.trim().is_empty() || code.len() > MAX_CODE_BYTES {
                return Err(TaxError::InvalidRule("invalid exemption code"));
            }
            if exemption_codes.insert(code.as_str(), true).is_some() {
                return Err(TaxError::InvalidRule("exemption codes must be unique"));
            }
        }
        if let Some(threshold) = &component.threshold {
            if threshold.amount_atomic < 0 || threshold.amount_atomic > MAX_ATOMIC_AMOUNT {
                return Err(TaxError::InvalidRule(
                    "threshold must fit non-negative Solana atomic units",
                ));
            }
            if component.price_mode == PriceMode::Inclusive
                && threshold.inclusive_equation.is_none()
            {
                return Err(TaxError::UnsupportedThreshold(
                    "inclusive threshold equations require an explicit rule version",
                ));
            }
            if component.price_mode == PriceMode::Exclusive
                && threshold.inclusive_equation.is_some()
            {
                return Err(TaxError::InvalidRule(
                    "inclusive threshold equations require inclusive price mode",
                ));
            }
        }
        if let RateSpec::Progressive(brackets) = &component.rate {
            validate_brackets(brackets)?;
        }
    }
    Ok(())
}

fn validate_brackets(brackets: &[Bracket]) -> Result<(), TaxError> {
    if brackets.is_empty() || brackets[0].lower_atomic != 0 {
        return Err(TaxError::InvalidRule(
            "progressive brackets must start at zero",
        ));
    }
    let mut expected_lower = 0;
    for (index, bracket) in brackets.iter().enumerate() {
        if bracket.lower_atomic != expected_lower
            || bracket.lower_atomic < 0
            || bracket.lower_atomic > MAX_ATOMIC_AMOUNT
        {
            return Err(TaxError::InvalidRule(
                "progressive brackets must be ordered and contiguous",
            ));
        }
        if let Some(upper) = bracket.upper_atomic {
            if upper <= bracket.lower_atomic || upper > MAX_ATOMIC_AMOUNT {
                return Err(TaxError::InvalidRule(
                    "progressive bracket upper bound must exceed lower bound and fit Solana u64 atomic units",
                ));
            }
            expected_lower = upper;
        } else if index + 1 != brackets.len() {
            return Err(TaxError::InvalidRule(
                "only the final progressive bracket may be open-ended",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn flat_rule(id: &str, base: TaxBase, rate: Rate, price_mode: PriceMode) -> TaxComponentRule {
        TaxComponentRule {
            id: id.to_string(),
            label: id.to_string(),
            base,
            rate: RateSpec::Flat(rate),
            price_mode,
            credit_atomic: 0,
            floor_atomic: None,
            cap_atomic: None,
            rounding: RoundingSpec {
                mode: RoundingMode::HalfUp,
                scope: RoundingScope::Component,
                quantum_atomic: 1,
            },
            threshold: None,
            exemption_codes: Vec::new(),
        }
    }

    fn facts(gross_atomic: i128, net_atomic: i128) -> TaxFacts {
        TaxFacts {
            gross_atomic,
            net_atomic,
            atomic_decimals: 2,
            currency: "USD".to_string(),
            jurisdiction: "US-CA".to_string(),
            jurisdiction_resolution: None,
            fx_conversion: None,
            exemption_codes: Vec::new(),
            cumulative_period: None,
            applicability: None,
        }
    }

    fn applicability_decision(
        facts_digest: Digest32,
        treatment: tax_applicability::Treatment,
    ) -> tax_applicability::ApplicabilityDecision {
        tax_applicability::ApplicabilityDecision {
            schema: tax_applicability::APPLICABILITY_SCHEMA_V1.to_string(),
            policy_id: "policy-us-ca-digital".to_string(),
            policy_version: 1,
            policy_digest: [9; 32],
            facts_digest,
            treatment,
            selected_rule_id: Some("taxable-digital-service".to_string()),
            matched_rule_ids: vec!["taxable-digital-service".to_string()],
            evaluated_at_unix: 1_700_000_100,
        }
    }

    fn applicability_rule() -> RuleSet {
        RuleSet {
            id: "applicability-binding".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).expect("test rate"),
                PriceMode::Exclusive,
            )],
        }
    }

    #[test]
    fn applicability_context_is_bound_to_facts_and_replay_result() {
        let mut transaction = facts(1_100, 1_000);
        let base_digest = transaction
            .pre_applicability_digest()
            .expect("base facts digest");
        transaction.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::Taxable,
        ));

        let result = calculate(&applicability_rule(), &transaction).expect("calculation");

        assert_eq!(result.applicability, transaction.applicability);
        assert_ne!(result.facts_digest, base_digest);
        result
            .verify_replay(&applicability_rule(), &transaction)
            .expect("applicability replay");
    }

    #[test]
    fn applicability_binding_rejects_a_decision_for_different_facts() {
        let mut transaction = facts(1_100, 1_000);
        transaction.applicability = Some(applicability_decision(
            [8; 32],
            tax_applicability::Treatment::Taxable,
        ));

        assert_eq!(
            calculate(&applicability_rule(), &transaction),
            Err(TaxError::ApplicabilityFactsDigestMismatch)
        );
    }

    #[test]
    fn unresolved_applicability_fails_closed_before_arithmetic() {
        let mut transaction = facts(1_100, 1_000);
        let base_digest = transaction
            .pre_applicability_digest()
            .expect("base facts digest");
        transaction.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::Unresolved,
        ));

        assert_eq!(
            calculate(&applicability_rule(), &transaction),
            Err(TaxError::UnresolvedApplicability)
        );
    }

    #[test]
    fn zero_rated_treatment_preserves_taxable_base_without_collecting_tax() {
        let mut transaction = facts(1_100, 1_000);
        let base_digest = transaction
            .pre_applicability_digest()
            .expect("base facts digest");
        transaction.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::ZeroRated,
        ));

        let result = calculate(&applicability_rule(), &transaction).expect("zero-rated result");

        assert_eq!(result.total_tax_atomic, 0);
        assert_eq!(result.total_net_atomic, 1_100);
        assert!(result.components.is_empty());
        assert_eq!(
            result.treatment_outcome,
            TreatmentOutcome {
                treatment: tax_applicability::Treatment::ZeroRated,
                arithmetic_tax_atomic: 0,
                collected_tax_atomic: 0,
                self_assessed_tax_atomic: 0,
                withheld_tax_atomic: 0,
                taxable_base_atomic: 1_100,
            }
        );
    }

    #[test]
    fn reverse_charge_reports_self_assessed_tax_without_reducing_seller_net() {
        let mut transaction = facts(1_100, 1_000);
        let base_digest = transaction
            .pre_applicability_digest()
            .expect("base facts digest");
        transaction.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::ReverseCharge,
        ));

        let result = calculate(&applicability_rule(), &transaction).expect("reverse-charge result");

        assert_eq!(result.total_tax_atomic, 0);
        assert_eq!(result.total_net_atomic, 1_100);
        assert_eq!(result.treatment_outcome.arithmetic_tax_atomic, 100);
        assert_eq!(result.treatment_outcome.collected_tax_atomic, 0);
        assert_eq!(result.treatment_outcome.self_assessed_tax_atomic, 100);
        assert_eq!(result.treatment_outcome.withheld_tax_atomic, 0);
        assert_eq!(result.components[0].tax_atomic, 0);
        result
            .verify_replay(&applicability_rule(), &transaction)
            .expect("reverse-charge replay");
    }

    #[test]
    fn withholding_reduces_seller_net_and_records_the_withheld_amount() {
        let mut transaction = facts(1_100, 1_000);
        let base_digest = transaction
            .pre_applicability_digest()
            .expect("base facts digest");
        transaction.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::Withholding,
        ));

        let result = calculate(&applicability_rule(), &transaction).expect("withholding result");

        assert_eq!(result.total_tax_atomic, 100);
        assert_eq!(result.total_net_atomic, 1_000);
        assert_eq!(result.treatment_outcome.arithmetic_tax_atomic, 100);
        assert_eq!(result.treatment_outcome.collected_tax_atomic, 100);
        assert_eq!(result.treatment_outcome.self_assessed_tax_atomic, 0);
        assert_eq!(result.treatment_outcome.withheld_tax_atomic, 100);
    }

    #[test]
    fn exempt_and_out_of_scope_treatments_have_no_authoritative_tax_graph() {
        for treatment in [
            tax_applicability::Treatment::Exempt,
            tax_applicability::Treatment::OutOfScope,
        ] {
            let mut transaction = facts(1_100, 1_000);
            let base_digest = transaction
                .pre_applicability_digest()
                .expect("base facts digest");
            transaction.applicability = Some(applicability_decision(base_digest, treatment));

            let result = calculate(&applicability_rule(), &transaction).expect("non-tax result");

            assert_eq!(result.total_tax_atomic, 0);
            assert_eq!(result.total_net_atomic, 1_100);
            assert!(result.components.is_empty());
            assert_eq!(result.treatment_outcome.treatment, treatment);
            assert_eq!(result.treatment_outcome.taxable_base_atomic, 0);
        }
    }

    #[test]
    fn document_applicability_context_binds_to_document_replay() {
        let rule = independent_document_rule(RoundingScope::Line);
        let mut document = document_facts(vec![line("line-1", 1_100)]);
        let base_digest = document
            .pre_applicability_digest()
            .expect("base document digest");
        document.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::Taxable,
        ));

        let result = calculate_document(&rule, &document).expect("document calculation");

        assert_eq!(result.applicability, document.applicability);
        assert_ne!(result.facts_digest, base_digest);
        result
            .verify_replay(&rule, &document)
            .expect("document applicability replay");
    }

    #[test]
    fn document_reverse_charge_preserves_line_conservation_and_self_assessed_total() {
        let rule = independent_document_rule(RoundingScope::Line);
        let mut document = document_facts(vec![line("line-1", 1_100), line("line-2", 550)]);
        let base_digest = document
            .pre_applicability_digest()
            .expect("base document digest");
        document.applicability = Some(applicability_decision(
            base_digest,
            tax_applicability::Treatment::ReverseCharge,
        ));

        let result = calculate_document(&rule, &document).expect("document reverse-charge");

        assert_eq!(result.gross_atomic, 1_650);
        assert_eq!(result.total_tax_atomic, 0);
        assert_eq!(result.total_net_atomic, 1_650);
        assert_eq!(result.treatment_outcome.self_assessed_tax_atomic, 1_100);
        assert!(
            result.lines.iter().all(
                |line| line.total_tax_atomic == 0 && line.total_net_atomic == line.gross_atomic
            )
        );
        result
            .verify_replay(&rule, &document)
            .expect("document reverse-charge replay");
    }

    #[test]
    fn treatment_semantics_match_the_versioned_golden_vector() {
        #[derive(Deserialize)]
        struct Vector {
            schema: String,
            cases: Vec<VectorCase>,
        }
        #[derive(Deserialize)]
        struct VectorCase {
            treatment: Treatment,
            gross_atomic: String,
            arithmetic_tax_atomic: String,
            expected: TreatmentExpected,
        }
        #[derive(Deserialize)]
        struct TreatmentExpected {
            collected_tax_atomic: String,
            self_assessed_tax_atomic: String,
            withheld_tax_atomic: String,
            taxable_base_atomic: String,
        }

        let vector: Vector =
            serde_json::from_str(include_str!("../../../test-vectors/tax-treatment-v1.json"))
                .expect("treatment vector");
        assert_eq!(vector.schema, "solana-tax-protocol/tax-treatment/v1");
        for case in vector.cases {
            let gross_atomic = case.gross_atomic.parse().expect("gross atomic");
            let arithmetic_tax_atomic = case
                .arithmetic_tax_atomic
                .parse()
                .expect("arithmetic tax atomic");
            let expected = TreatmentOutcome {
                treatment: case.treatment,
                arithmetic_tax_atomic,
                collected_tax_atomic: case
                    .expected
                    .collected_tax_atomic
                    .parse()
                    .expect("collected tax atomic"),
                self_assessed_tax_atomic: case
                    .expected
                    .self_assessed_tax_atomic
                    .parse()
                    .expect("self-assessed tax atomic"),
                withheld_tax_atomic: case
                    .expected
                    .withheld_tax_atomic
                    .parse()
                    .expect("withheld tax atomic"),
                taxable_base_atomic: case
                    .expected
                    .taxable_base_atomic
                    .parse()
                    .expect("taxable base atomic"),
            };
            assert_eq!(
                treatment_outcome(case.treatment, gross_atomic, arithmetic_tax_atomic)
                    .expect("treatment outcome"),
                expected
            );
        }
    }

    #[test]
    fn versioned_rule_document_round_trips_without_changing_rule_digest() {
        let rule_set = RuleSet {
            id: "document-test".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let rule_digest = rule_set.canonical_digest().unwrap();
        let document = RuleSetDocument::new(rule_set.clone());
        assert_eq!(document.schema, RULE_SET_DOCUMENT_SCHEMA_V1);
        document.validate().unwrap();

        let encoded = serde_json::to_vec(&document).unwrap();
        let decoded: RuleSetDocument = serde_json::from_slice(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.rule_set, rule_set);
        assert_eq!(decoded.rule_digest().unwrap(), rule_digest);
    }

    #[test]
    fn versioned_rule_document_rejects_unsupported_schema() {
        let rule_set = RuleSet {
            id: "document-test".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let mut document = RuleSetDocument::new(rule_set);
        document.schema = "solana-tax-protocol/tax-rule/v3".to_string();
        assert_eq!(
            document.validate(),
            Err(TaxError::InvalidRuleDocumentSchema)
        );
    }

    #[test]
    fn calculates_simple_percentage() {
        let rule = RuleSet {
            id: "simple".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let result = calculate(&rule, &facts(1_100, 1_000)).unwrap();
        assert_eq!(result.gross_atomic, 1_100);
        assert_eq!(result.total_tax_atomic, 100);
        assert_eq!(result.total_net_atomic, 1_000);
    }

    #[test]
    fn calculates_progressive_brackets() {
        let rule = RuleSet {
            id: "progressive".to_string(),
            version: 1,
            components: vec![TaxComponentRule {
                id: "income-tax".to_string(),
                label: "Income tax".to_string(),
                base: TaxBase::single(BaseReference::Net),
                rate: RateSpec::Progressive(vec![
                    Bracket {
                        lower_atomic: 0,
                        upper_atomic: Some(1_000),
                        rate: Rate::new(10, 100).unwrap(),
                    },
                    Bracket {
                        lower_atomic: 1_000,
                        upper_atomic: None,
                        rate: Rate::new(20, 100).unwrap(),
                    },
                ]),
                price_mode: PriceMode::Exclusive,
                credit_atomic: 0,
                floor_atomic: None,
                cap_atomic: None,
                rounding: RoundingSpec {
                    mode: RoundingMode::HalfUp,
                    scope: RoundingScope::Component,
                    quantum_atomic: 1,
                },
                threshold: None,
                exemption_codes: Vec::new(),
            }],
        };
        let result = calculate(&rule, &facts(1_700, 1_500)).unwrap();
        assert_eq!(result.total_tax_atomic, 200);
    }

    #[test]
    fn calculates_tax_on_tax_with_dependency_order() {
        let rule = RuleSet {
            id: "cascading".to_string(),
            version: 1,
            components: vec![
                flat_rule(
                    "first",
                    TaxBase::single(BaseReference::Net),
                    Rate::new(10, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
                flat_rule(
                    "second",
                    TaxBase {
                        sources: vec![
                            BaseReference::Net,
                            BaseReference::Component("first".to_string()),
                        ],
                        floor_atomic: None,
                        cap_atomic: None,
                    },
                    Rate::new(5, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
            ],
        };
        let result = calculate(&rule, &facts(1_155, 1_000)).unwrap();
        assert_eq!(result.total_tax_atomic, 155);
        assert_eq!(result.components[1].base_atomic, 1_100);
    }

    #[test]
    fn settlement_scope_rounds_once_and_allocates_residual_by_canonical_id() {
        let rule = RuleSet {
            id: "settlement-rounding".to_string(),
            version: 1,
            components: vec![
                TaxComponentRule {
                    id: "a".to_string(),
                    label: "A".to_string(),
                    base: TaxBase::single(BaseReference::Net),
                    rate: RateSpec::Flat(Rate::new(1, 3).unwrap()),
                    price_mode: PriceMode::Exclusive,
                    credit_atomic: 0,
                    floor_atomic: None,
                    cap_atomic: None,
                    rounding: RoundingSpec {
                        mode: RoundingMode::HalfUp,
                        scope: RoundingScope::Settlement,
                        quantum_atomic: 1,
                    },
                    threshold: None,
                    exemption_codes: Vec::new(),
                },
                TaxComponentRule {
                    id: "b".to_string(),
                    label: "B".to_string(),
                    base: TaxBase::single(BaseReference::Net),
                    rate: RateSpec::Flat(Rate::new(1, 3).unwrap()),
                    price_mode: PriceMode::Exclusive,
                    credit_atomic: 0,
                    floor_atomic: None,
                    cap_atomic: None,
                    rounding: RoundingSpec {
                        mode: RoundingMode::HalfUp,
                        scope: RoundingScope::Settlement,
                        quantum_atomic: 1,
                    },
                    threshold: None,
                    exemption_codes: Vec::new(),
                },
            ],
        };

        let result = calculate(&rule, &facts(1, 1)).unwrap();
        assert_eq!(result.total_tax_atomic, 1);
        assert_eq!(result.total_net_atomic, 0);
        assert_eq!(result.components[0].id, "a");
        assert_eq!(result.components[0].clamped_tax_atomic, 0);
        assert_eq!(result.components[0].tax_atomic, 1);
        assert_eq!(result.components[1].tax_atomic, 0);
    }

    #[test]
    fn settlement_scope_rejects_mixed_or_dependent_rounding_graphs() {
        let mut settlement = flat_rule(
            "settlement",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        settlement.rounding.scope = RoundingScope::Settlement;
        let mut component = flat_rule(
            "component",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        component.rounding.scope = RoundingScope::Component;
        let mixed = RuleSet {
            id: "mixed-rounding".to_string(),
            version: 1,
            components: vec![settlement.clone(), component.clone()],
        };
        assert!(
            matches!(mixed.compile(), Err(TaxError::InvalidRule(message)) if message.contains("same rounding scope"))
        );

        let mut dependent = settlement;
        dependent.id = "dependent".to_string();
        dependent.base = TaxBase::single(BaseReference::Component("component".to_string()));
        let mut settlement_component = component;
        settlement_component.rounding.scope = RoundingScope::Settlement;
        let dependent_rule = RuleSet {
            id: "dependent-rounding".to_string(),
            version: 1,
            components: vec![settlement_component, dependent],
        };
        assert!(
            matches!(dependent_rule.compile(), Err(TaxError::InvalidRule(message)) if message.contains("component dependencies"))
        );
    }

    #[test]
    fn component_rounding_uses_declared_atomic_quantum() {
        let mut rule = flat_rule(
            "quantized",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        rule.rounding.quantum_atomic = 10;

        let result = calculate(
            &RuleSet {
                id: "component-quantum".to_string(),
                version: 1,
                components: vec![rule],
            },
            &facts(100, 100),
        )
        .unwrap();

        assert_eq!(result.total_tax_atomic, 30);
        assert_eq!(result.components[0].tax_atomic % 10, 0);
    }

    #[test]
    fn settlement_rounding_allocates_residual_in_declared_atomic_quantum() {
        let mut first = flat_rule(
            "a",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        first.rounding.scope = RoundingScope::Settlement;
        first.rounding.quantum_atomic = 10;
        let mut second = first.clone();
        second.id = "b".to_string();

        let result = calculate(
            &RuleSet {
                id: "settlement-quantum".to_string(),
                version: 1,
                components: vec![first, second],
            },
            &facts(100, 100),
        )
        .unwrap();

        assert_eq!(result.total_tax_atomic, 70);
        assert_eq!(result.components[0].tax_atomic, 40);
        assert_eq!(result.components[1].tax_atomic, 30);
        assert!(
            result
                .components
                .iter()
                .all(|component| component.tax_atomic % 10 == 0)
        );
    }

    #[test]
    fn rejects_non_positive_or_mixed_rounding_quantum() {
        let mut invalid = flat_rule(
            "invalid-quantum",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        invalid.rounding.quantum_atomic = 0;
        assert!(matches!(
            (RuleSet {
                id: "invalid-quantum".to_string(),
                version: 1,
                components: vec![invalid],
            })
            .compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("quantum")
        ));

        let mut first = flat_rule(
            "a",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        first.rounding.scope = RoundingScope::Settlement;
        first.rounding.quantum_atomic = 10;
        let mut second = first.clone();
        second.id = "b".to_string();
        second.rounding.quantum_atomic = 100;
        assert!(matches!(
            (RuleSet {
                id: "mixed-quantum".to_string(),
                version: 1,
                components: vec![first, second],
            })
            .compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("atomic quanta")
        ));

        let mut incompatible_clamp = flat_rule(
            "incompatible-clamp",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        incompatible_clamp.rounding.quantum_atomic = 10;
        incompatible_clamp.floor_atomic = Some(3);
        assert!(matches!(
            (RuleSet {
                id: "incompatible-clamp".to_string(),
                version: 1,
                components: vec![incompatible_clamp],
            })
            .compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("exact multiples")
        ));
    }

    fn document_facts(lines: Vec<TaxLineFacts>) -> TaxDocumentFacts {
        TaxDocumentFacts {
            id: "invoice-1".to_string(),
            atomic_decimals: 2,
            currency: "USD".to_string(),
            jurisdiction: "US-CA".to_string(),
            jurisdiction_resolution: None,
            fx_conversion: None,
            cumulative_period: None,
            applicability: None,
            lines,
        }
    }

    fn line(id: &str, amount: i128) -> TaxLineFacts {
        TaxLineFacts {
            id: id.to_string(),
            gross_atomic: amount,
            net_atomic: amount,
            exemption_codes: Vec::new(),
        }
    }

    #[test]
    fn atomic_scale_is_required_in_canonical_facts_and_bounded() {
        let mut transaction = facts(1_100, 1_000);
        let digest = transaction.canonical_digest().unwrap();
        transaction.atomic_decimals = 6;
        assert_ne!(digest, transaction.canonical_digest().unwrap());

        transaction.atomic_decimals = 10;
        assert!(
            matches!(transaction.validate(), Err(TaxError::InvalidFacts(message)) if message.contains("atomic decimal scale"))
        );
    }

    #[test]
    fn calculated_result_carries_authoritative_atomic_scale() {
        let rule = RuleSet {
            id: "scale-result".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let mut transaction = facts(1_100, 1_000);
        transaction.atomic_decimals = 6;
        let result = calculate(&rule, &transaction).unwrap();
        assert_eq!(result.atomic_decimals, 6);
        let encoded = serde_json::to_vec(&result).unwrap();
        let decoded: CalculationResult = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.atomic_decimals, 6);
    }

    fn independent_document_rule(scope: RoundingScope) -> RuleSet {
        let mut first = flat_rule(
            "a",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 3).unwrap(),
            PriceMode::Exclusive,
        );
        first.rounding.scope = scope;
        let mut second = first.clone();
        second.id = "b".to_string();
        RuleSet {
            id: "document-rounding".to_string(),
            version: 1,
            components: vec![first, second],
        }
    }

    #[test]
    fn line_scope_rounds_each_line_and_orders_results_by_line_id() {
        let result = calculate_document(
            &independent_document_rule(RoundingScope::Line),
            &document_facts(vec![line("line-b", 1), line("line-a", 1)]),
        )
        .unwrap();

        assert_eq!(result.total_tax_atomic, 2);
        assert_eq!(result.total_net_atomic, 0);
        assert_eq!(
            result
                .lines
                .iter()
                .map(|line| line.id.as_str())
                .collect::<Vec<_>>(),
            vec!["line-a", "line-b"]
        );
        assert_eq!(result.lines[0].total_tax_atomic, 1);
        assert_eq!(result.lines[1].total_tax_atomic, 1);
        assert_eq!(result.lines[0].components[0].tax_atomic, 1);
        result
            .verify_replay(
                &independent_document_rule(RoundingScope::Line),
                &document_facts(vec![line("line-a", 1), line("line-b", 1)]),
            )
            .unwrap();
    }

    #[test]
    fn invoice_scope_rounds_once_and_allocates_by_line_then_component_id() {
        let result = calculate_document(
            &independent_document_rule(RoundingScope::Invoice),
            &document_facts(vec![line("line-b", 1), line("line-a", 1)]),
        )
        .unwrap();

        assert_eq!(result.total_tax_atomic, 1);
        assert_eq!(result.total_net_atomic, 1);
        assert_eq!(result.lines[0].id, "line-a");
        assert_eq!(result.lines[0].total_tax_atomic, 1);
        assert_eq!(result.lines[1].total_tax_atomic, 0);
        assert_eq!(result.lines[0].components[0].tax_atomic, 1);
        assert!(
            result
                .lines
                .iter()
                .flat_map(|line| line.components.iter())
                .all(|component| component.tax_atomic >= 0)
        );
    }

    #[test]
    fn document_rounding_rejects_non_conserving_tiny_line() {
        let result = calculate_document(
            &independent_document_rule(RoundingScope::Invoice),
            &document_facts(vec![
                line("line-00", 1),
                line("line-01", 2_305),
                line("line-02", 4_162),
            ]),
        );

        assert!(matches!(
            result,
            Err(TaxError::InvalidFacts(message))
                if message.contains("line tax cannot exceed gross amount")
        ));
    }

    #[test]
    fn document_digest_is_line_order_independent_but_line_identity_bound() {
        let first = document_facts(vec![line("line-a", 10), line("line-b", 20)]);
        let second = document_facts(vec![line("line-b", 20), line("line-a", 10)]);
        assert_eq!(
            first.canonical_digest().unwrap(),
            second.canonical_digest().unwrap()
        );

        let renamed = document_facts(vec![line("line-a", 10), line("line-c", 20)]);
        assert_ne!(
            first.canonical_digest().unwrap(),
            renamed.canonical_digest().unwrap()
        );
    }

    #[test]
    fn document_and_aggregate_scope_boundaries_fail_closed() {
        let document = document_facts(vec![line("line-a", 1)]);
        let line_rule = independent_document_rule(RoundingScope::Line);
        assert!(matches!(
            calculate(&line_rule, &facts(1, 1)),
            Err(TaxError::UnsupportedDocumentScope(_))
        ));

        let aggregate_rule = independent_document_rule(RoundingScope::Settlement);
        assert!(matches!(
            calculate_document(&aggregate_rule, &document),
            Err(TaxError::UnsupportedDocumentScope(_))
        ));

        let mut duplicate = document_facts(vec![line("line-a", 1), line("line-a", 1)]);
        assert!(matches!(
            duplicate.canonical_digest(),
            Err(TaxError::InvalidFacts(message)) if message.contains("line ids")
        ));
        duplicate.lines[1].id = "line-b".to_string();
        duplicate.lines[1].gross_atomic = 0;
        assert!(matches!(
            duplicate.canonical_digest(),
            Err(TaxError::InvalidFacts(message)) if message.contains("positive")
        ));
    }

    #[test]
    fn omitted_rounding_scope_deserializes_as_component_scope() {
        let document = serde_json::json!({
            "mode": "Floor"
        });
        let rounding: RoundingSpec = serde_json::from_value(document).unwrap();
        assert_eq!(rounding.scope, RoundingScope::Component);
        assert_eq!(rounding.quantum_atomic, 1);
        assert_eq!(
            serde_json::to_value(rounding).unwrap(),
            serde_json::json!({ "mode": "Floor" })
        );
    }

    #[test]
    fn non_default_rounding_quantum_is_bound_to_the_rule_digest() {
        let mut default_rule = RuleSet {
            id: "quantum-digest".to_string(),
            version: 1,
            components: vec![flat_rule(
                "tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(1, 10).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let default_digest = default_rule.canonical_digest().unwrap();
        default_rule.components[0].rounding.quantum_atomic = 10;
        assert_ne!(default_digest, default_rule.canonical_digest().unwrap());
    }

    #[test]
    fn calculates_inclusive_tax() {
        let rule = RuleSet {
            id: "inclusive".to_string(),
            version: 1,
            components: vec![flat_rule(
                "vat",
                TaxBase::single(BaseReference::Gross),
                Rate::new(10, 100).unwrap(),
                PriceMode::Inclusive,
            )],
        };
        let result = calculate(&rule, &facts(1_100, 1_000)).unwrap();
        assert_eq!(result.total_tax_atomic, 100);
    }

    #[test]
    fn applies_credit_floor_cap_and_truncation() {
        let mut component = flat_rule(
            "capped",
            TaxBase::single(BaseReference::Net),
            Rate::new(3, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.credit_atomic = 1;
        component.floor_atomic = Some(3);
        component.cap_atomic = Some(5);
        component.rounding.mode = RoundingMode::Floor;
        let rule = RuleSet {
            id: "clamps".to_string(),
            version: 1,
            components: vec![component],
        };
        let result = calculate(&rule, &facts(101, 101)).unwrap();
        assert_eq!(result.total_tax_atomic, 3);
    }

    #[test]
    fn rejects_dependency_cycles() {
        let mut first = flat_rule(
            "first",
            TaxBase::single(BaseReference::Component("second".to_string())),
            Rate::new(1, 100).unwrap(),
            PriceMode::Exclusive,
        );
        first.label = "First".to_string();
        let second = flat_rule(
            "second",
            TaxBase::single(BaseReference::Component("first".to_string())),
            Rate::new(1, 100).unwrap(),
            PriceMode::Exclusive,
        );
        let rule = RuleSet {
            id: "cycle".to_string(),
            version: 1,
            components: vec![first, second],
        };
        assert!(matches!(
            calculate(&rule, &facts(100, 100)),
            Err(TaxError::DependencyCycle(_))
        ));
    }

    #[test]
    fn compiles_dependencies_before_dependents() {
        let dependent = flat_rule(
            "second",
            TaxBase::single(BaseReference::Component("first".to_string())),
            Rate::new(5, 100).unwrap(),
            PriceMode::Exclusive,
        );
        let dependency = flat_rule(
            "first",
            TaxBase::single(BaseReference::Net),
            Rate::new(10, 100).unwrap(),
            PriceMode::Exclusive,
        );
        let rule = RuleSet {
            id: "ordered".to_string(),
            version: 1,
            components: vec![dependent, dependency],
        };

        let compiled = rule.compile().unwrap();
        let order = compiled
            .evaluation_order
            .iter()
            .map(|index| compiled.rule_set.components[*index].id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(order, ["first", "second"]);
        assert_eq!(
            compiled
                .calculate(&facts(1_155, 1_000))
                .unwrap()
                .total_tax_atomic,
            105
        );
    }

    #[test]
    fn rejects_duplicate_component_ids_before_graph_compilation() {
        let rule = RuleSet {
            id: "duplicate-components".to_string(),
            version: 1,
            components: vec![
                flat_rule(
                    "same",
                    TaxBase::single(BaseReference::Net),
                    Rate::new(1, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
                flat_rule(
                    "same",
                    TaxBase::single(BaseReference::Gross),
                    Rate::new(2, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
            ],
        };

        assert!(matches!(
            rule.compile(),
            Err(TaxError::InvalidRule("component ids must be unique"))
        ));
    }

    #[test]
    fn classifies_legacy_additive_bases_for_the_integer_fast_path() {
        assert!(is_legacy_additive_base(&TaxBase::single(
            BaseReference::Net
        )));
        assert!(is_legacy_additive_base(&TaxBase {
            sources: vec![
                BaseReference::Gross,
                BaseReference::Component("state".to_string())
            ],
            floor_atomic: None,
            cap_atomic: None,
        }));
        assert!(!is_legacy_additive_base(&TaxBase::weighted(
            WeightedBaseSource::Net,
            1,
            2,
        )));
        assert!(!is_legacy_additive_base(
            &TaxBase::single(BaseReference::Net).with_bounds(Some(0), None,)
        ));
    }

    #[test]
    fn rejects_duplicate_tax_base_sources_before_calculation() {
        let rule = RuleSet {
            id: "duplicate-base-source".to_string(),
            version: 1,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![BaseReference::Net, BaseReference::Net],
                    floor_atomic: None,
                    cap_atomic: None,
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };

        assert!(matches!(
            rule.compile(),
            Err(TaxError::InvalidRule(message))
                if message.contains("base sources must be distinct")
        ));
    }

    #[test]
    fn calculates_an_exact_weighted_tax_base() {
        let rule = RuleSet {
            id: "weighted-base".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase::weighted(WeightedBaseSource::Net, 1, 2),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };

        let result = calculate(&rule, &facts(200, 200)).unwrap();

        assert_eq!(result.components[0].base_atomic, 100);
        assert_eq!(result.total_tax_atomic, 10);
    }

    #[test]
    fn calculates_an_explicit_subtractive_tax_base() {
        let rule = RuleSet {
            id: "subtractive-base".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![
                        BaseReference::Gross,
                        BaseReference::weighted(WeightedBaseSource::Net, -1, 1),
                    ],
                    floor_atomic: None,
                    cap_atomic: None,
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };

        let result = calculate(&rule, &facts(1_000, 300)).unwrap();

        assert_eq!(result.components[0].base_atomic, 700);
        assert_eq!(result.total_tax_atomic, 70);
    }

    #[test]
    fn calculates_tax_base_floor_and_cap_before_tax_rate() {
        let floor_rule = RuleSet {
            id: "bounded-floor".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![
                        BaseReference::Gross,
                        BaseReference::weighted(WeightedBaseSource::Net, -1, 1),
                    ],
                    floor_atomic: Some(500),
                    cap_atomic: None,
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let floor_result = calculate(&floor_rule, &facts(1_000, 700)).unwrap();
        assert_eq!(floor_result.components[0].base_atomic, 500);
        assert_eq!(floor_result.total_tax_atomic, 50);

        let cap_rule = RuleSet {
            id: "bounded-cap".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![BaseReference::Gross],
                    floor_atomic: None,
                    cap_atomic: Some(400),
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let cap_result = calculate(&cap_rule, &facts(1_000, 1_000)).unwrap();
        assert_eq!(cap_result.components[0].base_atomic, 400);
        assert_eq!(cap_result.total_tax_atomic, 40);
    }

    #[test]
    fn applies_base_floor_then_cap_and_allows_flooring_negative_adjustment() {
        let rule = RuleSet {
            id: "bounded-order".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![
                        BaseReference::Gross,
                        BaseReference::weighted(WeightedBaseSource::Net, -1, 1),
                    ],
                    floor_atomic: Some(200),
                    cap_atomic: Some(250),
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let result = calculate(&rule, &facts(1_000, 700)).unwrap();
        assert_eq!(result.components[0].base_atomic, 250);
        assert_eq!(result.total_tax_atomic, 25);

        let negative_rule = RuleSet {
            id: "bounded-negative".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![
                        BaseReference::Gross,
                        BaseReference::weighted(WeightedBaseSource::Net, -1, 1),
                    ],
                    floor_atomic: Some(0),
                    cap_atomic: None,
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let negative_result = calculate(&negative_rule, &facts(100, 200)).unwrap();
        assert_eq!(negative_result.components[0].base_atomic, 0);
        assert_eq!(negative_result.total_tax_atomic, 0);
    }

    #[test]
    fn rejects_invalid_tax_base_bounds_and_legacy_v1_bound_envelope() {
        let invalid = RuleSet {
            id: "invalid-bounds".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![BaseReference::Net],
                    floor_atomic: Some(500),
                    cap_atomic: Some(400),
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        assert!(matches!(
            invalid.compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("base floor cannot exceed cap")
        ));

        let bounded = RuleSet {
            id: "bounded-envelope".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![BaseReference::Net],
                    floor_atomic: Some(1),
                    cap_atomic: None,
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let mut legacy_document = RuleSetDocument::new(bounded);
        legacy_document.schema = RULE_SET_DOCUMENT_SCHEMA_V1.to_string();
        assert_eq!(
            legacy_document.validate(),
            Err(TaxError::InvalidRuleDocumentSchema)
        );
    }

    proptest! {
        #[test]
        fn bounded_tax_base_stays_within_declared_bounds(
            raw_base in 0i128..=10_000,
            first_bound in 0i128..=10_000,
            second_bound in 0i128..=10_000,
        ) {
            let floor = first_bound.min(second_bound);
            let cap = first_bound.max(second_bound);
            let rule = RuleSet {
                id: "bounded-property".to_string(),
                version: 2,
                components: vec![flat_rule(
                    "tax",
                    TaxBase {
                        sources: vec![BaseReference::Gross],
                        floor_atomic: Some(floor),
                        cap_atomic: Some(cap),
                    },
                    Rate::new(0, 1).unwrap(),
                    PriceMode::Exclusive,
                )],
            };
            let result = calculate(&rule, &facts(raw_base, raw_base)).unwrap();
            prop_assert!(result.components[0].base_atomic >= floor);
            prop_assert!(result.components[0].base_atomic <= cap);
        }
    }

    #[test]
    fn evaluates_weighted_component_dependencies_before_dependents() {
        let rule = RuleSet {
            id: "weighted-dependent-base".to_string(),
            version: 2,
            components: vec![
                flat_rule(
                    "first",
                    TaxBase::single(BaseReference::Net),
                    Rate::new(10, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
                flat_rule(
                    "second",
                    TaxBase {
                        sources: vec![BaseReference::weighted(
                            WeightedBaseSource::Component("first".to_string()),
                            1,
                            2,
                        )],
                        floor_atomic: None,
                        cap_atomic: None,
                    },
                    Rate::new(10, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
            ],
        };

        let result = calculate(&rule, &facts(1_100, 1_000)).unwrap();

        assert_eq!(result.components[1].base_atomic, 50);
        assert_eq!(result.total_tax_atomic, 105);
    }

    #[test]
    fn rejects_a_weighted_base_that_is_not_integral_in_atomic_units() {
        let rule = RuleSet {
            id: "fractional-base".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase::weighted(WeightedBaseSource::Net, 1, 2),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };

        assert!(matches!(
            calculate(&rule, &facts(101, 101)),
            Err(TaxError::NonIntegralTaxBase)
        ));
    }

    #[test]
    fn rejects_invalid_weighted_coefficients_and_legacy_v1_envelopes() {
        let mut zero_numerator = RuleSet {
            id: "zero-weight".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase::weighted(WeightedBaseSource::Net, 0, 1),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        assert!(matches!(
            zero_numerator.compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("coefficient must be non-zero")
        ));

        let negative_denominator = RuleSet {
            id: "negative-denominator".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase::weighted(WeightedBaseSource::Net, 1, -1),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        assert!(matches!(
            negative_denominator.compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("denominator must be positive")
        ));

        let overflowed_numerator = RuleSet {
            id: "overflowed-numerator".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase::weighted(WeightedBaseSource::Net, MAX_ATOMIC_AMOUNT + 1, 1),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        assert!(matches!(
            overflowed_numerator.compile(),
            Err(TaxError::InvalidRule(message)) if message.contains("numerator must fit")
        ));

        zero_numerator.components[0].base = TaxBase::weighted(WeightedBaseSource::Net, 1, 2);
        let mut legacy_document = RuleSetDocument::new(zero_numerator);
        legacy_document.schema = RULE_SET_DOCUMENT_SCHEMA_V1.to_string();
        assert_eq!(
            legacy_document.validate(),
            Err(TaxError::InvalidRuleDocumentSchema)
        );
    }

    #[test]
    fn rejects_duplicate_source_identity_after_weighting() {
        let rule = RuleSet {
            id: "duplicate-weighted-base".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase {
                    sources: vec![
                        BaseReference::Net,
                        BaseReference::weighted(WeightedBaseSource::Net, -1, 1),
                    ],
                    floor_atomic: None,
                    cap_atomic: None,
                },
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };

        assert!(matches!(
            rule.compile(),
            Err(TaxError::InvalidRule(message))
                if message.contains("base sources must be distinct")
        ));
    }

    #[test]
    fn selects_the_minimum_rule_document_envelope_for_linear_bases() {
        let legacy = RuleSet {
            id: "legacy-envelope".to_string(),
            version: 1,
            components: vec![flat_rule(
                "tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let weighted = RuleSet {
            id: "linear-envelope".to_string(),
            version: 2,
            components: vec![flat_rule(
                "tax",
                TaxBase::weighted(WeightedBaseSource::Net, 1, 2),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };

        assert_eq!(
            RuleSetDocument::new(legacy).schema,
            RULE_SET_DOCUMENT_SCHEMA_V1
        );
        assert_eq!(
            RuleSetDocument::new(weighted).schema,
            RULE_SET_DOCUMENT_SCHEMA_V2
        );
    }

    #[test]
    fn rejects_amounts_that_cannot_be_settled_on_solana() {
        let rule = RuleSet {
            id: "bounded".to_string(),
            version: 1,
            components: vec![flat_rule(
                "tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(1, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        assert!(matches!(
            calculate(&rule, &facts(MAX_ATOMIC_AMOUNT + 1, MAX_ATOMIC_AMOUNT + 1)),
            Err(TaxError::InvalidFacts(_))
        ));
    }

    #[test]
    fn handles_zero_and_exact_u64_maximum_with_unit_rate_boundaries() {
        let zero_rule = RuleSet {
            id: "zero-rate".to_string(),
            version: 1,
            components: vec![flat_rule(
                "tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(0, 1).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let maximum_facts = facts(MAX_ATOMIC_AMOUNT, MAX_ATOMIC_AMOUNT);
        let zero_result = calculate(&zero_rule, &maximum_facts).unwrap();
        assert_eq!(zero_result.total_tax_atomic, 0);
        assert_eq!(zero_result.total_net_atomic, MAX_ATOMIC_AMOUNT);

        let unit_rule = RuleSet {
            id: "unit-rate".to_string(),
            version: 1,
            components: vec![flat_rule(
                "tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(1, 1).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let unit_result = calculate(&unit_rule, &maximum_facts).unwrap();
        assert_eq!(unit_result.total_tax_atomic, MAX_ATOMIC_AMOUNT);
        assert_eq!(unit_result.total_net_atomic, 0);
    }

    #[test]
    fn rejects_invalid_rate_and_clamp_boundaries() {
        assert!(matches!(Rate::new(-1, 1), Err(TaxError::InvalidRule(_))));
        assert!(matches!(Rate::new(1, 0), Err(TaxError::InvalidRule(_))));

        let mut component = flat_rule(
            "invalid-clamp",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.floor_atomic = Some(2);
        component.cap_atomic = Some(1);
        let rule = RuleSet {
            id: "invalid-clamp".to_string(),
            version: 1,
            components: vec![component],
        };
        assert!(matches!(
            calculate(&rule, &facts(100, 100)),
            Err(TaxError::InvalidRule(_))
        ));
    }

    #[test]
    fn rounds_exact_ties_according_to_each_declared_mode() {
        let half = Rational::new(5, 2).unwrap();
        assert_eq!(half.round(RoundingMode::Floor).unwrap(), 2);
        assert_eq!(half.round(RoundingMode::Ceil).unwrap(), 3);
        assert_eq!(half.round(RoundingMode::HalfUp).unwrap(), 3);
        assert_eq!(half.round(RoundingMode::HalfEven).unwrap(), 2);
        assert_eq!(half.round(RoundingMode::AwayFromZero).unwrap(), 3);

        let odd_half = Rational::new(3, 2).unwrap();
        assert_eq!(odd_half.round(RoundingMode::HalfEven).unwrap(), 2);
    }

    #[test]
    fn rejects_progressive_brackets_outside_settlement_bounds() {
        let mut component = flat_rule(
            "tax",
            TaxBase::single(BaseReference::Net),
            Rate::new(1, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.rate = RateSpec::Progressive(vec![Bracket {
            lower_atomic: 0,
            upper_atomic: Some(MAX_ATOMIC_AMOUNT + 1),
            rate: Rate::new(1, 100).unwrap(),
        }]);
        let rule = RuleSet {
            id: "bounded-brackets".to_string(),
            version: 1,
            components: vec![component],
        };
        assert!(matches!(
            calculate(&rule, &facts(100, 100)),
            Err(TaxError::InvalidRule(_))
        ));
    }

    #[test]
    fn applies_per_transaction_exempt_up_to_threshold() {
        let mut component = flat_rule(
            "thresholded",
            TaxBase::single(BaseReference::Net),
            Rate::new(10, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.threshold = Some(ThresholdRule {
            amount_atomic: 1_000,
            basis: ThresholdBasis::PerTransaction,
            mode: ThresholdMode::ExemptUpTo,
            inclusive_equation: None,
        });
        let result = calculate(
            &RuleSet {
                id: "threshold".to_string(),
                version: 1,
                components: vec![component],
            },
            &facts(1_500, 1_500),
        )
        .unwrap();
        assert_eq!(result.total_tax_atomic, 50);
        assert_eq!(result.components[0].effective_base_atomic, 500);
        assert_eq!(result.components[0].threshold_atomic, Some(1_000));
    }

    #[test]
    fn trigger_threshold_taxes_the_full_base_only_after_crossing() {
        let mut component = flat_rule(
            "triggered",
            TaxBase::single(BaseReference::Net),
            Rate::new(10, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.threshold = Some(ThresholdRule {
            amount_atomic: 1_000,
            basis: ThresholdBasis::PerTransaction,
            mode: ThresholdMode::TriggerAt,
            inclusive_equation: None,
        });
        let rule = RuleSet {
            id: "trigger".to_string(),
            version: 1,
            components: vec![component],
        };
        assert_eq!(
            calculate(&rule, &facts(999, 999)).unwrap().total_tax_atomic,
            0
        );
        assert_eq!(
            calculate(&rule, &facts(1_001, 1_001))
                .unwrap()
                .total_tax_atomic,
            100
        );
    }

    #[test]
    fn cumulative_threshold_consumes_a_bound_snapshot() {
        let mut component = flat_rule(
            "cumulative",
            TaxBase::single(BaseReference::Net),
            Rate::new(10, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.threshold = Some(ThresholdRule {
            amount_atomic: 1_000,
            basis: ThresholdBasis::CumulativePeriod,
            mode: ThresholdMode::ExemptUpTo,
            inclusive_equation: None,
        });
        let rule = RuleSet {
            id: "cumulative".to_string(),
            version: 1,
            components: vec![component],
        };
        let mut transaction_facts = facts(1_200, 200);
        transaction_facts.cumulative_period = Some(CumulativePeriodSnapshot {
            period_id: "2026-Q3".to_string(),
            cumulative_base_atomic: 900,
            evidence_digest: [7; 32],
        });
        let result = calculate(&rule, &transaction_facts).unwrap();
        assert_eq!(result.total_tax_atomic, 10);
        assert_eq!(result.components[0].effective_base_atomic, 100);
        assert_eq!(result.components[0].threshold_atomic, Some(1_000));
        assert!(matches!(
            calculate(&rule, &facts(1_200, 200)),
            Err(TaxError::MissingCumulativeSnapshot)
        ));
    }

    #[test]
    fn matching_exemption_code_zeroes_only_the_declared_component() {
        let mut component = flat_rule(
            "exemptible",
            TaxBase::single(BaseReference::Net),
            Rate::new(10, 100).unwrap(),
            PriceMode::Exclusive,
        );
        component.exemption_codes = vec!["RESALE".to_string()];
        let rule = RuleSet {
            id: "exemption".to_string(),
            version: 1,
            components: vec![component],
        };
        let mut transaction_facts = facts(1_000, 1_000);
        transaction_facts.exemption_codes = vec!["RESALE".to_string()];
        let result = calculate(&rule, &transaction_facts).unwrap();
        assert_eq!(result.total_tax_atomic, 0);
        assert!(result.components[0].exemption_applied);
    }

    #[test]
    fn rejects_inclusive_threshold_without_an_explicit_equation() {
        let mut component = flat_rule(
            "inclusive-threshold",
            TaxBase::single(BaseReference::Gross),
            Rate::new(10, 100).unwrap(),
            PriceMode::Inclusive,
        );
        component.threshold = Some(ThresholdRule {
            amount_atomic: 100,
            basis: ThresholdBasis::PerTransaction,
            mode: ThresholdMode::ExemptUpTo,
            inclusive_equation: None,
        });
        let rule = RuleSet {
            id: "inclusive-threshold".to_string(),
            version: 1,
            components: vec![component],
        };
        assert!(matches!(
            calculate(&rule, &facts(1_100, 1_000)),
            Err(TaxError::UnsupportedThreshold(_))
        ));
    }

    #[test]
    fn calculates_inclusive_gross_threshold_with_an_explicit_equation() {
        let rule: RuleSet = serde_json::from_value(serde_json::json!({
            "id": "inclusive-gross-threshold",
            "version": 1,
            "components": [{
                "id": "vat",
                "label": "VAT",
                "base": {"sources": ["Gross"]},
                "rate": {"Flat": {"numerator": "10", "denominator": "100"}},
                "price_mode": "Inclusive",
                "credit_atomic": "0",
                "floor_atomic": null,
                "cap_atomic": null,
                "rounding": {"mode": "HalfUp"},
                "threshold": {
                    "amount_atomic": "1100",
                    "basis": "PerTransaction",
                    "mode": "ExemptUpTo",
                    "inclusive_equation": "GrossExcess"
                },
                "exemption_codes": []
            }]
        }))
        .unwrap();

        let result = calculate(&rule, &facts(1_200, 1_200)).unwrap();
        assert_eq!(result.total_tax_atomic, 9);
        assert_eq!(result.total_net_atomic, 1_191);
        assert_eq!(result.components[0].effective_base_atomic, 100);
    }

    #[test]
    fn cumulative_snapshot_requires_nonzero_evidence_and_bounded_values() {
        let mut transaction_facts = facts(1_100, 1_000);
        transaction_facts.cumulative_period = Some(CumulativePeriodSnapshot {
            period_id: "2026-Q3".to_string(),
            cumulative_base_atomic: 1_000,
            evidence_digest: [0; 32],
        });
        assert!(matches!(
            transaction_facts.canonical_digest(),
            Err(TaxError::InvalidFacts(_))
        ));
    }

    #[test]
    fn canonical_facts_digest_encodes_cumulative_evidence_exactly_once() {
        let mut transaction_facts = facts(1_200, 200);
        transaction_facts.cumulative_period = Some(CumulativePeriodSnapshot {
            period_id: "2026-Q3".to_string(),
            cumulative_base_atomic: 900,
            evidence_digest: [7; 32],
        });
        assert_eq!(
            transaction_facts.canonical_digest().unwrap(),
            [
                0x68, 0x9a, 0x54, 0xef, 0x81, 0x94, 0x60, 0x76, 0xde, 0x61, 0x43, 0xf1, 0xcf, 0xa0,
                0x37, 0xce, 0xaf, 0xb8, 0x68, 0x83, 0x78, 0x49, 0x92, 0x90, 0x77, 0x4f, 0xea, 0x7a,
                0xd1, 0xb2, 0x41, 0x93,
            ]
        );
    }

    #[test]
    fn jurisdiction_resolution_is_bound_to_facts_and_replay() {
        let policy = jurisdiction::ResolutionPolicy {
            id: "digital-commerce".to_string(),
            version: 1,
            ordered_sources: vec![jurisdiction::EvidenceSource::ShippingDestination],
            conflict_mode: jurisdiction::ConflictMode::HighestPrecedence,
        };
        let candidates = vec![jurisdiction::JurisdictionCandidate {
            jurisdiction_code: "US-CA".to_string(),
            source: jurisdiction::EvidenceSource::ShippingDestination,
            evidence_digest: [7; 32],
        }];
        let resolution = policy.resolve(&candidates).unwrap();
        let mut transaction_facts = facts(1_100, 1_000);
        transaction_facts.jurisdiction_resolution = Some(resolution.clone());
        let bound_digest = transaction_facts.canonical_digest().unwrap();
        let rule = RuleSet {
            id: "bound-resolution".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let result = calculate(&rule, &transaction_facts).unwrap();
        assert_eq!(result.jurisdiction_resolution, Some(resolution));
        result.verify_replay(&rule, &transaction_facts).unwrap();

        let mut untrusted_facts = facts(1_100, 1_000);
        untrusted_facts.jurisdiction = "CALLER-SUPPLIED".to_string();
        let bound_result =
            calculate_with_jurisdiction_resolution(&rule, &untrusted_facts, &policy, &candidates)
                .unwrap();
        assert_eq!(bound_result.jurisdiction, "US-CA");
        assert!(bound_result.jurisdiction_resolution.is_some());

        let mut tampered_facts = transaction_facts.clone();
        tampered_facts
            .jurisdiction_resolution
            .as_mut()
            .unwrap()
            .evidence_digest = [8; 32];
        assert_ne!(bound_digest, tampered_facts.canonical_digest().unwrap());
        assert_eq!(
            result.verify_replay(&rule, &tampered_facts),
            Err(TaxError::ReplayMismatch)
        );
    }

    fn fx_evidence(
        source_currency: &str,
        target_currency: &str,
        numerator: i128,
        denominator: i128,
    ) -> FxConversionEvidence {
        FxConversionEvidence {
            source_currency: source_currency.to_string(),
            target_currency: target_currency.to_string(),
            numerator,
            denominator,
            rounding_mode: RoundingMode::HalfUp,
            quantum_atomic: 1,
            provider: "regulated-fx".to_string(),
            quote_id: "quote-2026-10-02-1".to_string(),
            observed_at_unix: 1_759_000_000,
            evidence_digest: [7; 32],
        }
    }

    #[test]
    fn calculation_without_fx_reuses_borrowed_facts_on_the_hot_path() {
        let transaction_facts = facts(1_000, 500);
        let normalized = transaction_facts.normalized_for_calculation().unwrap();
        assert!(matches!(normalized, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn calculation_with_fx_owns_normalized_facts() {
        let mut transaction_facts = facts(1_000, 500);
        transaction_facts.currency = "EUR".to_string();
        transaction_facts.fx_conversion = Some(fx_evidence("USD", "EUR", 3, 2));
        let normalized = transaction_facts.normalized_for_calculation().unwrap();
        assert!(matches!(normalized, std::borrow::Cow::Owned(_)));
    }

    #[test]
    fn exact_fx_evidence_normalizes_before_tax_and_replays() {
        let rule = RuleSet {
            id: "fx-rule".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let mut transaction_facts = facts(1_000, 500);
        transaction_facts.currency = "EUR".to_string();
        transaction_facts.fx_conversion = Some(fx_evidence("USD", "EUR", 3, 2));

        let result = calculate(&rule, &transaction_facts).unwrap();
        assert_eq!(result.currency, "EUR");
        assert_eq!(result.gross_atomic, 1_500);
        assert_eq!(result.total_tax_atomic, 75);
        assert_eq!(result.total_net_atomic, 1_425);
        assert_eq!(result.fx_conversion, transaction_facts.fx_conversion);
        result.verify_replay(&rule, &transaction_facts).unwrap();
    }

    #[test]
    fn document_fx_evidence_normalizes_each_line_and_preserves_result_binding() {
        let mut document = document_facts(vec![line("line-a", 1_000)]);
        document.currency = "EUR".to_string();
        document.fx_conversion = Some(fx_evidence("USD", "EUR", 3, 2));
        let result =
            calculate_document(&independent_document_rule(RoundingScope::Line), &document).unwrap();
        assert_eq!(result.gross_atomic, 1_500);
        assert_eq!(result.fx_conversion, document.fx_conversion);
        result
            .verify_replay(&independent_document_rule(RoundingScope::Line), &document)
            .unwrap();
    }

    #[test]
    fn fx_conversion_rounding_and_cumulative_snapshot_use_declared_policy() {
        let rule = RuleSet {
            id: "fx-rounding".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(1, 10).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let mut evidence = fx_evidence("JPY", "USD", 1, 2);
        evidence.quantum_atomic = 10;
        let mut transaction_facts = facts(10, 10);
        transaction_facts.currency = "USD".to_string();
        transaction_facts.fx_conversion = Some(evidence);
        transaction_facts.cumulative_period = Some(CumulativePeriodSnapshot {
            period_id: "2026-Q4".to_string(),
            cumulative_base_atomic: 11,
            evidence_digest: [8; 32],
        });

        let result = calculate(&rule, &transaction_facts).unwrap();
        assert_eq!(result.gross_atomic, 10);
        assert_eq!(result.total_net_atomic, 9);
        assert_eq!(result.total_tax_atomic, 1);
    }

    #[test]
    fn malformed_or_out_of_range_fx_evidence_fails_closed() {
        let mut transaction_facts = facts(100, 100);
        transaction_facts.currency = "EUR".to_string();
        let mut evidence = fx_evidence("USD", "USD", 1, 1);
        transaction_facts.fx_conversion = Some(evidence.clone());
        assert!(matches!(
            transaction_facts.validate(),
            Err(TaxError::InvalidFacts(message)) if message.contains("source and target")
        ));

        evidence = fx_evidence("USD", "EUR", 2, 1);
        transaction_facts.fx_conversion = Some(evidence);
        transaction_facts.gross_atomic = MAX_ATOMIC_AMOUNT;
        transaction_facts.net_atomic = MAX_ATOMIC_AMOUNT;
        assert!(matches!(
            calculate(
                &RuleSet {
                    id: "fx-overflow".to_string(),
                    version: 1,
                    components: vec![flat_rule(
                        "sales-tax",
                        TaxBase::single(BaseReference::Net),
                        Rate::new(0, 1).unwrap(),
                        PriceMode::Exclusive,
                    )],
                },
                &transaction_facts
            ),
            Err(TaxError::FxConversionOutOfRange)
        ));
    }

    #[test]
    fn canonical_digests_are_stable_across_component_order() {
        let first = flat_rule(
            "first",
            TaxBase::single(BaseReference::Net),
            Rate::new(10, 100).unwrap(),
            PriceMode::Exclusive,
        );
        let second = flat_rule(
            "second",
            TaxBase::single(BaseReference::Gross),
            Rate::new(5, 100).unwrap(),
            PriceMode::Exclusive,
        );
        let left = RuleSet {
            id: "stable".to_string(),
            version: 1,
            components: vec![first.clone(), second.clone()],
        };
        let right = RuleSet {
            id: "stable".to_string(),
            version: 1,
            components: vec![second, first],
        };
        assert_eq!(
            left.canonical_digest().unwrap(),
            right.canonical_digest().unwrap()
        );
        assert_eq!(
            facts(1_100, 1_000).canonical_digest().unwrap(),
            facts(1_100, 1_000).canonical_digest().unwrap()
        );
    }

    #[test]
    fn compiled_rule_set_is_equivalent_to_uncached_calculation() {
        let rule = RuleSet {
            id: "compiled".to_string(),
            version: 1,
            components: vec![
                flat_rule(
                    "first",
                    TaxBase::single(BaseReference::Net),
                    Rate::new(10, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
                flat_rule(
                    "second",
                    TaxBase {
                        sources: vec![
                            BaseReference::Net,
                            BaseReference::Component("first".to_string()),
                        ],
                        floor_atomic: None,
                        cap_atomic: None,
                    },
                    Rate::new(5, 100).unwrap(),
                    PriceMode::Exclusive,
                ),
            ],
        };
        let compiled = rule.compile().unwrap();
        let facts = facts(1_155, 1_000);

        assert_eq!(
            compiled.calculate(&facts).unwrap(),
            calculate(&rule, &facts).unwrap()
        );
        assert_eq!(compiled.rule_digest(), rule.canonical_digest().unwrap());
        assert_eq!(compiled.rule_set(), &rule);
    }

    #[test]
    fn calculation_result_replays_only_with_the_same_engine_and_trace() {
        let rule = RuleSet {
            id: "replay".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let facts = facts(1_100, 1_000);
        let result = calculate(&rule, &facts).unwrap();

        assert_eq!(result.engine_version, ENGINE_VERSION);
        assert!(result.verify_replay(&rule, &facts).is_ok());

        let mut tampered = result.clone();
        tampered.components[0].tax_atomic += 1;
        assert_eq!(
            tampered.verify_replay(&rule, &facts),
            Err(TaxError::ReplayMismatch)
        );

        let mut old_engine = result;
        old_engine.engine_version = "0.0.0-old".to_string();
        assert!(matches!(
            old_engine.verify_replay(&rule, &facts),
            Err(TaxError::EngineVersionMismatch { .. })
        ));
    }

    #[test]
    fn legacy_result_without_engine_version_fails_replay_closed() {
        let rule = RuleSet {
            id: "legacy-replay".to_string(),
            version: 1,
            components: vec![flat_rule(
                "sales-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(10, 100).unwrap(),
                PriceMode::Exclusive,
            )],
        };
        let facts = facts(1_100, 1_000);
        let result = calculate(&rule, &facts).unwrap();
        let mut document = serde_json::to_value(result).unwrap();
        document
            .as_object_mut()
            .expect("calculation result serializes as an object")
            .remove("engine_version");
        let legacy: CalculationResult = serde_json::from_value(document).unwrap();
        assert!(matches!(
            legacy.verify_replay(&rule, &facts),
            Err(TaxError::EngineVersionMismatch { actual, .. }) if actual.is_empty()
        ));
    }

    #[test]
    fn serializes_atomic_values_as_decimal_strings() {
        let serialized =
            serde_json::to_string(&facts(9_007_199_254_740_993, 9_007_199_254_740_992)).unwrap();
        assert!(serialized.contains("\"gross_atomic\":\"9007199254740993\""));
        let round_trip: TaxFacts = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            round_trip,
            facts(9_007_199_254_740_993, 9_007_199_254_740_992)
        );
    }

    #[test]
    fn omitted_applicability_preserves_legacy_wire_shape_and_digest() {
        let legacy = facts(1_100, 1_000);
        let serialized = serde_json::to_value(&legacy).unwrap();

        assert!(serialized.get("applicability").is_none());
        assert_eq!(
            legacy.canonical_digest().unwrap(),
            legacy.pre_applicability_digest().unwrap()
        );

        let decoded: TaxFacts = serde_json::from_value(serialized).unwrap();
        assert_eq!(decoded, legacy);
    }

    #[derive(Deserialize)]
    struct GoldenVectorFile {
        schema: String,
        vectors: Vec<GoldenVector>,
    }

    #[derive(Deserialize)]
    struct GoldenVector {
        id: String,
        rule_set: RuleSet,
        facts: TaxFacts,
        expected: GoldenVectorExpected,
    }

    #[derive(Deserialize)]
    struct GoldenVectorExpected {
        #[serde(with = "serde_i128_string")]
        total_tax_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        total_net_atomic: AtomicAmount,
        components: Vec<GoldenComponentExpected>,
    }

    #[derive(Deserialize)]
    struct GoldenComponentExpected {
        id: String,
        #[serde(with = "serde_i128_string")]
        base_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        effective_base_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        tax_atomic: AtomicAmount,
        exemption_applied: bool,
    }

    fn assert_golden_vector_manifest(
        schema: &str,
        vectors: &[GoldenVector],
        required_ids: &[&str],
    ) {
        let actual_ids = vectors
            .iter()
            .map(|vector| vector.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            actual_ids.len(),
            vectors.len(),
            "{schema} golden vector IDs must be unique"
        );
        let required_ids = required_ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            actual_ids, required_ids,
            "{schema} golden vector manifest drifted"
        );
    }

    #[test]
    fn executes_the_versioned_global_tax_golden_vectors() {
        let file: GoldenVectorFile =
            serde_json::from_str(include_str!("../../../test-vectors/tax-engine-v1.json"))
                .expect("tax-engine golden vectors must be valid JSON");
        assert_eq!(file.schema, "solana-tax-protocol/tax-engine/v1");
        assert_golden_vector_manifest(
            &file.schema,
            &file.vectors,
            &[
                "simple-exclusive",
                "progressive-marginal",
                "tax-on-tax-cascade",
                "inclusive-price",
                "inclusive-gross-threshold",
                "credit-floor-cap-truncation",
                "cumulative-threshold",
                "exemption-and-rounding",
                "minimum-and-maximum",
                "ceil-rounding",
                "settlement-scope-largest-remainder",
                "settlement-scope-quantum-residual",
            ],
        );

        for vector in file.vectors {
            let result = calculate(&vector.rule_set, &vector.facts)
                .unwrap_or_else(|error| panic!("golden vector {} failed: {error}", vector.id));
            assert_eq!(
                result.total_tax_atomic, vector.expected.total_tax_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.total_net_atomic, vector.expected.total_net_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.components.len(),
                vector.expected.components.len(),
                "{}",
                vector.id
            );
            for expected in vector.expected.components {
                let actual = result
                    .components
                    .iter()
                    .find(|component| component.id == expected.id)
                    .unwrap_or_else(|| {
                        panic!("component {} missing in {}", expected.id, vector.id)
                    });
                assert_eq!(actual.base_atomic, expected.base_atomic, "{}", vector.id);
                assert_eq!(
                    actual.effective_base_atomic, expected.effective_base_atomic,
                    "{}",
                    vector.id
                );
                assert_eq!(actual.tax_atomic, expected.tax_atomic, "{}", vector.id);
                assert_eq!(
                    actual.exemption_applied, expected.exemption_applied,
                    "{}",
                    vector.id
                );
            }
        }
    }

    #[test]
    fn executes_the_v2_linear_tax_base_golden_vectors() {
        let file: GoldenVectorFile =
            serde_json::from_str(include_str!("../../../test-vectors/tax-engine-v2.json"))
                .expect("tax-engine v2 golden vectors must be valid JSON");
        assert_eq!(file.schema, "solana-tax-protocol/tax-engine/v2");
        assert_golden_vector_manifest(
            &file.schema,
            &file.vectors,
            &[
                "weighted-net-share",
                "subtractive-gross-adjustment",
                "bounded-adjusted-base",
                "capped-adjusted-base",
                "weighted-component-cascade",
            ],
        );

        for vector in file.vectors {
            let result = calculate(&vector.rule_set, &vector.facts)
                .unwrap_or_else(|error| panic!("golden vector {} failed: {error}", vector.id));
            assert_eq!(
                result.total_tax_atomic, vector.expected.total_tax_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.total_net_atomic, vector.expected.total_net_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.components.len(),
                vector.expected.components.len(),
                "{}",
                vector.id
            );
            for expected in vector.expected.components {
                let actual = result
                    .components
                    .iter()
                    .find(|component| component.id == expected.id)
                    .unwrap_or_else(|| {
                        panic!("component {} missing in {}", expected.id, vector.id)
                    });
                assert_eq!(actual.base_atomic, expected.base_atomic, "{}", vector.id);
                assert_eq!(
                    actual.effective_base_atomic, expected.effective_base_atomic,
                    "{}",
                    vector.id
                );
                assert_eq!(actual.tax_atomic, expected.tax_atomic, "{}", vector.id);
                assert_eq!(
                    actual.exemption_applied, expected.exemption_applied,
                    "{}",
                    vector.id
                );
            }
        }
    }

    #[derive(Deserialize)]
    struct DocumentGoldenVectorFile {
        schema: String,
        vectors: Vec<DocumentGoldenVector>,
    }

    #[derive(Deserialize)]
    struct DocumentGoldenVector {
        id: String,
        rule_set: RuleSet,
        facts: TaxDocumentFacts,
        expected: DocumentGoldenExpected,
    }

    #[derive(Deserialize)]
    struct DocumentGoldenExpected {
        #[serde(with = "serde_i128_string")]
        gross_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        total_tax_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        total_net_atomic: AtomicAmount,
        lines: Vec<DocumentGoldenLineExpected>,
    }

    #[derive(Deserialize)]
    struct DocumentGoldenLineExpected {
        id: String,
        #[serde(with = "serde_i128_string")]
        gross_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        total_tax_atomic: AtomicAmount,
        #[serde(with = "serde_i128_string")]
        total_net_atomic: AtomicAmount,
        components: Vec<DocumentGoldenComponentExpected>,
    }

    #[derive(Deserialize)]
    struct DocumentGoldenComponentExpected {
        id: String,
        #[serde(with = "serde_i128_string")]
        tax_atomic: AtomicAmount,
    }

    #[test]
    fn executes_the_versioned_tax_document_golden_vectors() {
        let file: DocumentGoldenVectorFile =
            serde_json::from_str(include_str!("../../../test-vectors/tax-document-v1.json"))
                .expect("tax-document golden vectors must be valid JSON");
        assert_eq!(file.schema, TAX_DOCUMENT_SCHEMA_V1);
        assert_eq!(file.vectors.len(), 2);

        for vector in file.vectors {
            let result = calculate_document(&vector.rule_set, &vector.facts)
                .unwrap_or_else(|error| panic!("golden document {} failed: {error}", vector.id));
            assert_eq!(
                result.gross_atomic, vector.expected.gross_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.total_tax_atomic, vector.expected.total_tax_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.total_net_atomic, vector.expected.total_net_atomic,
                "{}",
                vector.id
            );
            assert_eq!(
                result.lines.len(),
                vector.expected.lines.len(),
                "{}",
                vector.id
            );
            for expected_line in vector.expected.lines {
                let actual_line = result
                    .lines
                    .iter()
                    .find(|line| line.id == expected_line.id)
                    .unwrap_or_else(|| {
                        panic!("line {} missing in {}", expected_line.id, vector.id)
                    });
                assert_eq!(
                    actual_line.gross_atomic, expected_line.gross_atomic,
                    "{}",
                    vector.id
                );
                assert_eq!(
                    actual_line.total_tax_atomic, expected_line.total_tax_atomic,
                    "{}",
                    vector.id
                );
                assert_eq!(
                    actual_line.total_net_atomic, expected_line.total_net_atomic,
                    "{}",
                    vector.id
                );
                for expected_component in expected_line.components {
                    let actual_component = actual_line
                        .components
                        .iter()
                        .find(|component| component.id == expected_component.id)
                        .unwrap_or_else(|| {
                            panic!(
                                "component {} missing in line {} of {}",
                                expected_component.id, expected_line.id, vector.id
                            )
                        });
                    assert_eq!(
                        actual_component.tax_atomic, expected_component.tax_atomic,
                        "{}",
                        vector.id
                    );
                }
            }
        }
    }

    proptest! {
        #[test]
        fn simple_tax_preserves_gross_conservation_for_bounded_inputs(
            gross in 0i128..1_000_000i128,
            net_selector in 0i128..1_000_000i128,
            numerator in 0i128..101i128,
            denominator in 1i128..10_001i128,
        ) {
            let net = if gross == 0 { 0 } else { net_selector % (gross + 1) };
            let numerator = numerator.min(denominator);
            let rule = RuleSet {
                id: "property-simple".to_string(),
                version: 1,
                components: vec![flat_rule(
                    "tax",
                    TaxBase::single(BaseReference::Net),
                    Rate::new(numerator, denominator).expect("generated rate is valid"),
                    PriceMode::Exclusive,
                )],
            };
            let input = facts(gross, net);
            let direct = calculate(&rule, &input).expect("bounded rate cannot exceed gross");
            let compiled = rule.compile().expect("generated rule is valid");
            let cached = compiled.calculate(&input).expect("compiled rule is valid");
            prop_assert_eq!(&direct, &cached);
            prop_assert_eq!(direct.total_tax_atomic + direct.total_net_atomic, gross);
            prop_assert!(direct.total_tax_atomic >= 0);
            prop_assert!(direct.total_tax_atomic <= gross);
        }

        #[test]
        fn generated_u64_amounts_preserve_conservation_through_clamps(
            gross_u64 in any::<u64>(),
            net_selector in any::<u64>(),
            numerator in 0u64..=10_000u64,
            denominator in 1u64..=10_000u64,
            floor_selector in any::<u64>(),
            cap_selector in any::<u64>(),
        ) {
            let gross = i128::from(gross_u64);
            let net = i128::from(net_selector) % (gross + 1);
            let denominator = i128::from(denominator);
            let numerator = i128::from(numerator).min(denominator);
            let floor = i128::from(floor_selector) % (gross + 1);
            let cap = floor + (i128::from(cap_selector) % (gross - floor + 1));
            let mut component = flat_rule(
                "bounded-tax",
                TaxBase::single(BaseReference::Net),
                Rate::new(numerator, denominator).expect("generated rate is valid"),
                PriceMode::Exclusive,
            );
            component.floor_atomic = Some(floor);
            component.cap_atomic = Some(cap);
            let rule = RuleSet {
                id: "property-u64-boundary".to_string(),
                version: 1,
                components: vec![component],
            };
            let result = calculate(&rule, &facts(gross, net))
                .expect("bounded rate and clamp cannot exceed the gross amount");
            prop_assert_eq!(result.total_tax_atomic + result.total_net_atomic, gross);
            prop_assert!(result.total_tax_atomic >= 0);
            prop_assert!(result.total_tax_atomic <= gross);
        }

        #[test]
        fn document_rounding_preserves_conservation_for_bounded_line_sets(
            // The fixture has two independent 1/3 components. A one- or
            // two-unit line can round both components up and is therefore an
            // intentionally invalid conservation case, not a valid input for
            // this property. Rejection is covered by the boundary tests.
            amounts in prop::collection::vec(3i128..10_000i128, 1..=8),
        ) {
            let rule = independent_document_rule(RoundingScope::Invoice);
            let lines = amounts
                .iter()
                .enumerate()
                .map(|(index, amount)| line(&format!("line-{index:02}"), *amount))
                .collect::<Vec<_>>();
            let document = document_facts(lines);
            let result = calculate_document(&rule, &document)
                .expect("bounded invoice facts preserve document conservation");
            prop_assert_eq!(result.total_tax_atomic + result.total_net_atomic, result.gross_atomic);
            prop_assert!(result.total_tax_atomic >= 0);
            prop_assert!(result.total_tax_atomic <= result.gross_atomic);
            for line_result in result.lines {
                prop_assert_eq!(line_result.total_tax_atomic + line_result.total_net_atomic, line_result.gross_atomic);
                prop_assert!(line_result.total_tax_atomic >= 0);
            }
        }
    }
}
