#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

//! Evidence-bound tax applicability and classification domain.
//!
//! This crate deliberately does not perform arithmetic tax calculation. It
//! resolves bounded, versioned treatment context that a pure tax engine can
//! consume after the API has verified provider evidence and tenant scope.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

pub type Digest32 = [u8; 32];

pub const APPLICABILITY_SCHEMA_V1: &str = "solana-tax-protocol/tax-applicability/v1";
pub const MAX_POLICY_RULES: usize = 64;
pub const MAX_RULE_PREDICATES: usize = 16;
pub const MAX_MATCHED_RULES: usize = 64;
pub const MAX_FACTS_CODE_BYTES: usize = 128;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum FactKey {
    ProductCategory,
    SupplyType,
    CustomerClass,
    MerchantRegistration,
    CustomerRegistration,
    TransactionType,
}

impl FactKey {
    fn tag(&self) -> u8 {
        match self {
            Self::ProductCategory => 0,
            Self::SupplyType => 1,
            Self::CustomerClass => 2,
            Self::MerchantRegistration => 3,
            Self::CustomerRegistration => 4,
            Self::TransactionType => 5,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicabilityFacts {
    pub schema: String,
    pub product_category: String,
    pub supply_type: String,
    pub customer_class: String,
    pub merchant_registration: String,
    pub customer_registration: String,
    pub transaction_type: String,
    pub evidence_digest: Digest32,
    pub valid_until_unix: i64,
}

impl ApplicabilityFacts {
    pub fn validate(&self) -> Result<(), ApplicabilityError> {
        if self.schema != APPLICABILITY_SCHEMA_V1 {
            return Err(ApplicabilityError::UnsupportedSchema);
        }
        for value in [
            &self.product_category,
            &self.supply_type,
            &self.customer_class,
            &self.merchant_registration,
            &self.customer_registration,
            &self.transaction_type,
        ] {
            validate_code(value)?;
        }
        if self.evidence_digest == [0; 32] || self.valid_until_unix <= 0 {
            return Err(ApplicabilityError::InvalidFacts(
                "evidence digest and validity are required",
            ));
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<Digest32, ApplicabilityError> {
        self.validate()?;
        let mut bytes = Vec::new();
        text(&mut bytes, &self.schema)?;
        for value in [
            &self.product_category,
            &self.supply_type,
            &self.customer_class,
            &self.merchant_registration,
            &self.customer_registration,
            &self.transaction_type,
        ] {
            text(&mut bytes, value)?;
        }
        bytes.extend_from_slice(&self.evidence_digest);
        bytes.extend_from_slice(&self.valid_until_unix.to_le_bytes());
        Ok(Sha256::digest(bytes).into())
    }

    fn value(&self, key: &FactKey) -> &str {
        match key {
            FactKey::ProductCategory => &self.product_category,
            FactKey::SupplyType => &self.supply_type,
            FactKey::CustomerClass => &self.customer_class,
            FactKey::MerchantRegistration => &self.merchant_registration,
            FactKey::CustomerRegistration => &self.customer_registration,
            FactKey::TransactionType => &self.transaction_type,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Predicate {
    Equals { key: FactKey, value: String },
}

impl Predicate {
    pub fn equals(key: FactKey, value: impl Into<String>) -> Self {
        Self::Equals {
            key,
            value: value.into(),
        }
    }

    fn validate(&self) -> Result<(), ApplicabilityError> {
        match self {
            Self::Equals { value, .. } => validate_code(value),
        }
    }

    fn matches(&self, facts: &ApplicabilityFacts) -> bool {
        match self {
            Self::Equals { key, value } => facts.value(key) == value,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Treatment {
    Taxable,
    ZeroRated,
    Exempt,
    ReverseCharge,
    Withholding,
    OutOfScope,
    Unresolved,
}

impl Treatment {
    fn tag(self) -> u8 {
        match self {
            Self::Taxable => 0,
            Self::ZeroRated => 1,
            Self::Exempt => 2,
            Self::ReverseCharge => 3,
            Self::Withholding => 4,
            Self::OutOfScope => 5,
            Self::Unresolved => 6,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicabilityRule {
    pub id: String,
    pub priority: u32,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub predicates: Vec<Predicate>,
    pub treatment: Treatment,
}

impl ApplicabilityRule {
    fn validate(&self) -> Result<(), ApplicabilityError> {
        validate_code(&self.id)?;
        if self.priority == 0
            || self.effective_from_unix <= 0
            || self
                .effective_until_unix
                .is_some_and(|until| until <= self.effective_from_unix)
            || self.predicates.is_empty()
            || self.predicates.len() > MAX_RULE_PREDICATES
        {
            return Err(ApplicabilityError::InvalidPolicy(
                "rule priority, effective window, and predicate bounds are invalid",
            ));
        }
        let mut keys = BTreeSet::new();
        for predicate in &self.predicates {
            predicate.validate()?;
            let Predicate::Equals { key, .. } = predicate;
            if !keys.insert(key) {
                return Err(ApplicabilityError::InvalidPolicy(
                    "a rule cannot repeat an applicability fact key",
                ));
            }
        }
        Ok(())
    }

    fn is_active(&self, now_unix: i64) -> bool {
        self.effective_from_unix <= now_unix
            && self
                .effective_until_unix
                .is_none_or(|until| now_unix < until)
    }

    fn matches(&self, facts: &ApplicabilityFacts) -> bool {
        self.predicates
            .iter()
            .all(|predicate| predicate.matches(facts))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicabilityPolicy {
    pub schema: String,
    pub id: String,
    pub version: u32,
    pub jurisdiction: String,
    pub regime: String,
    pub effective_from_unix: i64,
    pub effective_until_unix: Option<i64>,
    pub rules: Vec<ApplicabilityRule>,
}

impl ApplicabilityPolicy {
    pub fn validate(&self) -> Result<(), ApplicabilityError> {
        if self.schema != APPLICABILITY_SCHEMA_V1 {
            return Err(ApplicabilityError::UnsupportedSchema);
        }
        for value in [&self.id, &self.jurisdiction, &self.regime] {
            validate_code(value)?;
        }
        if self.version == 0
            || self.effective_from_unix <= 0
            || self
                .effective_until_unix
                .is_some_and(|until| until <= self.effective_from_unix)
            || self.rules.len() > MAX_POLICY_RULES
        {
            return Err(ApplicabilityError::InvalidPolicy(
                "policy identity, effective window, or rule count is invalid",
            ));
        }
        let mut ids = BTreeSet::new();
        for rule in &self.rules {
            rule.validate()?;
            if !ids.insert(rule.id.as_str()) {
                return Err(ApplicabilityError::InvalidPolicy(
                    "applicability rule IDs must be unique",
                ));
            }
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<Digest32, ApplicabilityError> {
        self.validate()?;
        let mut bytes = Vec::new();
        text(&mut bytes, &self.schema)?;
        text(&mut bytes, &self.id)?;
        bytes.extend_from_slice(&self.version.to_le_bytes());
        text(&mut bytes, &self.jurisdiction)?;
        text(&mut bytes, &self.regime)?;
        bytes.extend_from_slice(&self.effective_from_unix.to_le_bytes());
        optional_i64(&mut bytes, self.effective_until_unix);
        let mut rules = self.rules.iter().collect::<Vec<_>>();
        rules.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.id.cmp(&right.id))
        });
        bytes.extend_from_slice(&(rules.len() as u32).to_le_bytes());
        for rule in rules {
            text(&mut bytes, &rule.id)?;
            bytes.extend_from_slice(&rule.priority.to_le_bytes());
            bytes.extend_from_slice(&rule.effective_from_unix.to_le_bytes());
            optional_i64(&mut bytes, rule.effective_until_unix);
            bytes.push(rule.treatment.tag());
            bytes.extend_from_slice(&(rule.predicates.len() as u32).to_le_bytes());
            for predicate in &rule.predicates {
                match predicate {
                    Predicate::Equals { key, value } => {
                        bytes.push(0);
                        bytes.push(key.tag());
                        text(&mut bytes, value)?;
                    }
                }
            }
        }
        Ok(Sha256::digest(bytes).into())
    }

    #[cfg(test)]
    fn test_policy() -> Self {
        Self {
            schema: APPLICABILITY_SCHEMA_V1.to_string(),
            id: "policy-us-ca-digital".to_string(),
            version: 1,
            jurisdiction: "US-CA".to_string(),
            regime: "sales-tax".to_string(),
            effective_from_unix: 1_600_000_000,
            effective_until_unix: None,
            rules: vec![ApplicabilityRule {
                id: "withholding-resident".to_string(),
                priority: 100,
                effective_from_unix: 1_600_000_000,
                effective_until_unix: None,
                predicates: vec![
                    Predicate::equals(FactKey::ProductCategory, "digital-service"),
                    Predicate::equals(FactKey::CustomerClass, "business"),
                ],
                treatment: Treatment::Withholding,
            }],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicabilityDecision {
    pub schema: String,
    pub policy_id: String,
    pub policy_version: u32,
    pub policy_digest: Digest32,
    pub facts_digest: Digest32,
    pub treatment: Treatment,
    pub selected_rule_id: Option<String>,
    pub matched_rule_ids: Vec<String>,
    pub evaluated_at_unix: i64,
}

impl ApplicabilityDecision {
    pub fn validate(&self) -> Result<(), ApplicabilityError> {
        if self.schema != APPLICABILITY_SCHEMA_V1
            || self.policy_id.trim().is_empty()
            || self.policy_id.len() > MAX_FACTS_CODE_BYTES
            || self.policy_version == 0
            || self.policy_digest == [0; 32]
            || self.facts_digest == [0; 32]
            || self.evaluated_at_unix <= 0
            || self.matched_rule_ids.len() > MAX_MATCHED_RULES
        {
            return Err(ApplicabilityError::InvalidDecision);
        }
        if let Some(selected_rule_id) = &self.selected_rule_id {
            validate_code(selected_rule_id)?;
            if !self
                .matched_rule_ids
                .iter()
                .any(|rule_id| rule_id == selected_rule_id)
            {
                return Err(ApplicabilityError::InvalidDecision);
            }
        }
        for rule_id in &self.matched_rule_ids {
            validate_code(rule_id)?;
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<Digest32, ApplicabilityError> {
        self.validate()?;
        let mut bytes = Vec::new();
        text(&mut bytes, &self.schema)?;
        text(&mut bytes, &self.policy_id)?;
        bytes.extend_from_slice(&self.policy_version.to_le_bytes());
        bytes.extend_from_slice(&self.policy_digest);
        bytes.extend_from_slice(&self.facts_digest);
        bytes.push(self.treatment.tag());
        match &self.selected_rule_id {
            Some(id) => {
                bytes.push(1);
                text(&mut bytes, id)?;
            }
            None => bytes.push(0),
        }
        bytes.extend_from_slice(&(self.matched_rule_ids.len() as u32).to_le_bytes());
        for id in &self.matched_rule_ids {
            text(&mut bytes, id)?;
        }
        bytes.extend_from_slice(&self.evaluated_at_unix.to_le_bytes());
        Ok(Sha256::digest(bytes).into())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ApplicabilityError {
    #[error("unsupported applicability schema")]
    UnsupportedSchema,
    #[error("invalid applicability facts: {0}")]
    InvalidFacts(&'static str),
    #[error("invalid applicability policy: {0}")]
    InvalidPolicy(&'static str),
    #[error("applicability evidence is stale")]
    EvidenceExpired,
    #[error("applicability policy is outside its effective window")]
    PolicyOutsideEffectiveWindow,
    #[error("applicability rules have ambiguous precedence")]
    AmbiguousPrecedence,
    #[error("applicability decision is invalid")]
    InvalidDecision,
    #[error("canonical applicability value is too long")]
    ValueTooLong,
}

pub fn evaluate(
    policy: &ApplicabilityPolicy,
    facts: &ApplicabilityFacts,
    now_unix: i64,
) -> Result<ApplicabilityDecision, ApplicabilityError> {
    if now_unix <= 0 {
        return Err(ApplicabilityError::InvalidFacts(
            "evaluation time is invalid",
        ));
    }
    policy.validate()?;
    facts.validate()?;
    if facts.valid_until_unix <= now_unix {
        return Err(ApplicabilityError::EvidenceExpired);
    }
    if policy.effective_from_unix > now_unix
        || policy
            .effective_until_unix
            .is_some_and(|until| now_unix >= until)
    {
        return Err(ApplicabilityError::PolicyOutsideEffectiveWindow);
    }

    let mut matches = policy
        .rules
        .iter()
        .filter(|rule| rule.is_active(now_unix) && rule.matches(facts))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    if matches
        .get(1)
        .is_some_and(|rule| rule.priority == matches[0].priority)
    {
        return Err(ApplicabilityError::AmbiguousPrecedence);
    }
    let selected = matches.first().map(|rule| rule.id.clone());
    let treatment = matches
        .first()
        .map_or(Treatment::Unresolved, |rule| rule.treatment);
    let facts_digest = facts.canonical_digest()?;
    let policy_digest = policy.canonical_digest()?;
    Ok(ApplicabilityDecision {
        schema: APPLICABILITY_SCHEMA_V1.to_string(),
        policy_id: policy.id.clone(),
        policy_version: policy.version,
        policy_digest,
        facts_digest,
        treatment,
        selected_rule_id: selected,
        matched_rule_ids: matches.into_iter().map(|rule| rule.id.clone()).collect(),
        evaluated_at_unix: now_unix,
    })
}

fn validate_code(value: &str) -> Result<(), ApplicabilityError> {
    if value.trim().is_empty()
        || value.len() > MAX_FACTS_CODE_BYTES
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(ApplicabilityError::ValueTooLong);
    }
    Ok(())
}

fn text(bytes: &mut Vec<u8>, value: &str) -> Result<(), ApplicabilityError> {
    validate_code(value)?;
    let length = u32::try_from(value.len()).map_err(|_| ApplicabilityError::ValueTooLong)?;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn optional_i64(bytes: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        None => bytes.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl ApplicabilityFacts {
        fn test_facts() -> Self {
            Self {
                schema: APPLICABILITY_SCHEMA_V1.to_string(),
                product_category: "digital-service".to_string(),
                supply_type: "cross-border".to_string(),
                customer_class: "business".to_string(),
                merchant_registration: "registered".to_string(),
                customer_registration: "unregistered".to_string(),
                transaction_type: "sale".to_string(),
                evidence_digest: [7; 32],
                valid_until_unix: 1_700_001_000,
            }
        }
    }

    #[test]
    fn selects_the_highest_precedence_matching_treatment() {
        let facts = ApplicabilityFacts::test_facts();
        let policy = ApplicabilityPolicy::test_policy();

        let decision = evaluate(&policy, &facts, 1_700_000_100).expect("decision");

        assert_eq!(decision.treatment, Treatment::Withholding);
        assert_eq!(
            decision.selected_rule_id,
            Some("withholding-resident".to_string())
        );
        assert_eq!(
            decision.matched_rule_ids,
            vec!["withholding-resident".to_string()]
        );
    }

    #[test]
    fn unresolved_context_is_not_silently_zero_tax() {
        let facts = ApplicabilityFacts::test_facts();
        let policy = ApplicabilityPolicy {
            rules: Vec::new(),
            ..ApplicabilityPolicy::test_policy()
        };

        let decision = evaluate(&policy, &facts, 1_700_000_100).expect("decision");

        assert_eq!(decision.treatment, Treatment::Unresolved);
        assert!(decision.selected_rule_id.is_none());
    }

    #[test]
    fn rejects_equal_precedence_matches_with_different_treatments() {
        let facts = ApplicabilityFacts::test_facts();
        let mut policy = ApplicabilityPolicy::test_policy();
        policy.rules.push(ApplicabilityRule {
            id: "conflict".to_string(),
            priority: 100,
            effective_from_unix: 1_600_000_000,
            effective_until_unix: None,
            predicates: vec![Predicate::equals(
                FactKey::ProductCategory,
                "digital-service",
            )],
            treatment: Treatment::Exempt,
        });

        assert_eq!(
            evaluate(&policy, &facts, 1_700_000_100),
            Err(ApplicabilityError::AmbiguousPrecedence)
        );
    }

    #[test]
    fn rejects_expired_evidence_before_rule_selection() {
        let mut facts = ApplicabilityFacts::test_facts();
        facts.valid_until_unix = 1_700_000_100;

        assert_eq!(
            evaluate(
                &ApplicabilityPolicy::test_policy(),
                &facts,
                facts.valid_until_unix
            ),
            Err(ApplicabilityError::EvidenceExpired)
        );
    }

    #[test]
    fn decision_rejects_a_selected_rule_not_present_in_matches() {
        let facts = ApplicabilityFacts::test_facts();
        let policy = ApplicabilityPolicy::test_policy();
        let mut decision = evaluate(&policy, &facts, 1_700_000_100).expect("decision");
        decision.selected_rule_id = Some("not-matched".to_string());

        assert_eq!(
            decision.canonical_digest(),
            Err(ApplicabilityError::InvalidDecision)
        );
    }

    #[test]
    fn decision_rejects_an_unbounded_match_list() {
        let facts = ApplicabilityFacts::test_facts();
        let policy = ApplicabilityPolicy::test_policy();
        let mut decision = evaluate(&policy, &facts, 1_700_000_100).expect("decision");
        decision.matched_rule_ids = (0..=MAX_MATCHED_RULES)
            .map(|index| format!("rule-{index}"))
            .collect();
        decision.selected_rule_id = None;

        assert_eq!(
            decision.canonical_digest(),
            Err(ApplicabilityError::InvalidDecision)
        );
    }

    #[test]
    fn executes_the_versioned_golden_vector() {
        #[derive(Deserialize)]
        struct GoldenVector {
            schema: String,
            facts: ApplicabilityFacts,
            policy: ApplicabilityPolicy,
            evaluated_at_unix: i64,
            expected: GoldenExpected,
        }

        #[derive(Deserialize)]
        struct GoldenExpected {
            treatment: Treatment,
            selected_rule_id: Option<String>,
            matched_rule_ids: Vec<String>,
        }

        let vector: GoldenVector = serde_json::from_str(include_str!(
            "../../../test-vectors/tax-applicability-v1.json"
        ))
        .expect("valid applicability golden vector");
        assert_eq!(vector.schema, APPLICABILITY_SCHEMA_V1);
        let decision = evaluate(&vector.policy, &vector.facts, vector.evaluated_at_unix)
            .expect("golden applicability decision");
        assert_eq!(decision.treatment, vector.expected.treatment);
        assert_eq!(decision.selected_rule_id, vector.expected.selected_rule_id);
        assert_eq!(decision.matched_rule_ids, vector.expected.matched_rule_ids);
        assert_ne!(
            decision.canonical_digest().expect("decision digest"),
            [0; 32]
        );
    }
}
