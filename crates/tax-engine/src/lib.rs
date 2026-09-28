//! Deterministic, replayable tax calculation domain logic.
//!
//! This crate intentionally has no network, clock, database, filesystem, RPC,
//! or wallet dependencies. All external facts are supplied as immutable input.

use jurisdiction::JurisdictionResolution;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

pub type AtomicAmount = i128;
pub type Digest32 = [u8; 32];
pub const MAX_ATOMIC_AMOUNT: AtomicAmount = u64::MAX as AtomicAmount;
pub const MAX_COMPONENTS: usize = 128;
pub const MAX_EXEMPTION_CODES: usize = 32;
pub const MAX_CODE_BYTES: usize = 64;
pub const MAX_PERIOD_ID_BYTES: usize = 128;
pub const RULE_SET_DOCUMENT_SCHEMA_V1: &str = "solana-tax-protocol/tax-rule/v1";
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoundingSpec {
    pub mode: RoundingMode,
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
    pub currency: String,
    pub jurisdiction: String,
    /// Server-owned resolution evidence. `None` is retained only for legacy
    /// records; new globally supported quote paths must require `Some`.
    #[serde(default)]
    pub jurisdiction_resolution: Option<JurisdictionResolution>,
    #[serde(default)]
    pub exemption_codes: Vec<String>,
    #[serde(default)]
    pub cumulative_period: Option<CumulativePeriodSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CumulativePeriodSnapshot {
    pub period_id: String,
    #[serde(with = "serde_i128_string")]
    pub cumulative_base_atomic: AtomicAmount,
    pub evidence_digest: Digest32,
}

impl TaxFacts {
    fn validate(&self) -> Result<(), TaxError> {
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
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<Digest32, TaxError> {
        self.validate()?;
        let mut encoder = CanonicalEncoder::new();
        encoder.i128(self.gross_atomic);
        encoder.i128(self.net_atomic);
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
        if let Some(resolution) = &self.jurisdiction_resolution {
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
        Ok(encoder.digest())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BaseReference {
    Gross,
    Net,
    Component(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaxBase {
    pub sources: Vec<BaseReference>,
}

impl TaxBase {
    pub fn single(source: BaseReference) -> Self {
        Self {
            sources: vec![source],
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThresholdRule {
    #[serde(with = "serde_i128_string")]
    pub amount_atomic: AtomicAmount,
    pub basis: ThresholdBasis,
    pub mode: ThresholdMode,
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
        Self {
            schema: RULE_SET_DOCUMENT_SCHEMA_V1.to_string(),
            rule_set,
        }
    }

    pub fn validate(&self) -> Result<(), TaxError> {
        if self.schema != RULE_SET_DOCUMENT_SCHEMA_V1 {
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

impl CompiledRuleSet {
    pub fn rule_set(&self) -> &RuleSet {
        &self.rule_set
    }

    pub fn rule_digest(&self) -> Digest32 {
        self.rule_digest
    }

    pub fn calculate(&self, facts: &TaxFacts) -> Result<CalculationResult, TaxError> {
        facts.validate()?;
        let facts_digest = facts.canonical_digest()?;
        let mut computed = BTreeMap::<String, AtomicAmount>::new();
        let mut results = BTreeMap::<String, ComponentResult>::new();
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
            let tax_atomic = with_cap.round(component.rounding.mode)?;
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
                clamped_tax_atomic: with_cap.round(RoundingMode::Floor)?,
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

        Ok(CalculationResult {
            rule_set_id: self.rule_set.id.clone(),
            rule_version: self.rule_set.version,
            engine_version: ENGINE_VERSION.to_string(),
            currency: facts.currency.clone(),
            jurisdiction: facts.jurisdiction.clone(),
            jurisdiction_resolution: facts.jurisdiction_resolution.clone(),
            facts_digest,
            rule_digest: self.rule_digest,
            gross_atomic: facts.gross_atomic,
            total_tax_atomic,
            total_net_atomic,
            components: results.into_values().collect(),
        })
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
pub struct CalculationResult {
    pub rule_set_id: String,
    pub rule_version: u32,
    // Missing versions are legacy evidence and must fail replay closed. New
    // calculations always populate this field explicitly below.
    #[serde(default)]
    pub engine_version: String,
    pub currency: String,
    pub jurisdiction: String,
    #[serde(default)]
    pub jurisdiction_resolution: Option<JurisdictionResolution>,
    pub facts_digest: Digest32,
    pub rule_digest: Digest32,
    #[serde(with = "serde_i128_string")]
    pub gross_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_tax_atomic: AtomicAmount,
    #[serde(with = "serde_i128_string")]
    pub total_net_atomic: AtomicAmount,
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

#[derive(Debug, Error, Eq, PartialEq)]
pub enum TaxError {
    #[error("invalid rule: {0}")]
    InvalidRule(&'static str),
    #[error("unsupported tax rule document schema")]
    InvalidRuleDocumentSchema,
    #[error("invalid facts: {0}")]
    InvalidFacts(&'static str),
    #[error("missing tax component: {0}")]
    MissingComponent(String),
    #[error("tax rule dependency cycle detected at component: {0}")]
    DependencyCycle(String),
    #[error("arithmetic overflow")]
    ArithmeticOverflow,
    #[error("unsupported inclusive calculation: {0}")]
    UnsupportedInclusive(&'static str),
    #[error("cumulative threshold requires an evidence snapshot")]
    MissingCumulativeSnapshot,
    #[error("unsupported threshold calculation: {0}")]
    UnsupportedThreshold(&'static str),
    #[error("calculation result was produced by engine version {actual}, expected {expected}")]
    EngineVersionMismatch { expected: String, actual: String },
    #[error("calculation replay does not match the stored result")]
    ReplayMismatch,
    #[error("jurisdiction resolution failed: {0}")]
    JurisdictionResolution(#[from] jurisdiction::JurisdictionError),
}

pub fn calculate(rule_set: &RuleSet, facts: &TaxFacts) -> Result<CalculationResult, TaxError> {
    rule_set.compile()?.calculate(facts)
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
        }
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
        if let BaseReference::Component(id) = dependency {
            let dependency_index = indices
                .get(id.as_str())
                .copied()
                .ok_or_else(|| TaxError::MissingComponent(id.clone()))?;
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
    base.sources.iter().try_fold(0_i128, |total, source| {
        let value = match source {
            BaseReference::Gross => facts.gross_atomic,
            BaseReference::Net => facts.net_atomic,
            BaseReference::Component(id) => computed
                .get(id)
                .copied()
                .ok_or_else(|| TaxError::MissingComponent(id.clone()))?,
        };
        total.checked_add(value).ok_or(TaxError::ArithmeticOverflow)
    })
}

fn apply_threshold(
    component: &TaxComponentRule,
    base_atomic: AtomicAmount,
    facts: &TaxFacts,
) -> Result<AtomicAmount, TaxError> {
    let Some(threshold) = &component.threshold else {
        return Ok(base_atomic);
    };
    if component.price_mode == PriceMode::Inclusive {
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
    let mut ids = HashMap::new();
    for component in &rule_set.components {
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
            if component.price_mode == PriceMode::Inclusive {
                return Err(TaxError::UnsupportedThreshold(
                    "inclusive threshold equations require an explicit rule version",
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
            },
            threshold: None,
            exemption_codes: Vec::new(),
        }
    }

    fn facts(gross_atomic: i128, net_atomic: i128) -> TaxFacts {
        TaxFacts {
            gross_atomic,
            net_atomic,
            currency: "USD".to_string(),
            jurisdiction: "US-CA".to_string(),
            jurisdiction_resolution: None,
            exemption_codes: Vec::new(),
            cumulative_period: None,
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
        document.schema = "solana-tax-protocol/tax-rule/v2".to_string();
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
                0x28, 0x7d, 0x5b, 0x04, 0xd4, 0x65, 0xdd, 0xb8, 0x1c, 0x71, 0xf8, 0xe2, 0x76, 0x0f,
                0x23, 0xcf, 0x5a, 0x6d, 0x06, 0x07, 0xa9, 0xfe, 0x53, 0xab, 0x66, 0x8d, 0x47, 0x2a,
                0xc8, 0x2a, 0xa7, 0x16,
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

    #[test]
    fn executes_the_versioned_global_tax_golden_vectors() {
        let file: GoldenVectorFile =
            serde_json::from_str(include_str!("../../../test-vectors/tax-engine-v1.json"))
                .expect("tax-engine golden vectors must be valid JSON");
        assert_eq!(file.schema, "solana-tax-protocol/tax-engine/v1");
        assert!(file.vectors.len() >= 8);

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
    }
}
