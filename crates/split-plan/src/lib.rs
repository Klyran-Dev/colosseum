#![deny(unsafe_code)]

use anchor_lang::prelude::Pubkey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use thiserror::Error;

pub const SPLIT_PLAN_SCHEMA: &str = "solana-tax-protocol/split-plan/v1";
pub const MAX_RECIPIENTS: usize = 8;
pub const BASIS_POINTS_DENOMINATOR: u16 = 10_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SplitRecipient {
    pub token_account: String,
    pub basis_points: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SplitPlan {
    pub merchant: String,
    pub mint: String,
    pub version: u64,
    pub recipients: Vec<SplitRecipient>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Allocation {
    pub token_account: String,
    pub amount_atomic: u64,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum SplitPlanError {
    #[error("invalid public key for {field}")]
    InvalidPublicKey { field: &'static str },
    #[error("split plan version must be positive")]
    InvalidVersion,
    #[error("split plan must contain between one and eight recipients")]
    InvalidRecipientCount,
    #[error("recipient basis points must be positive")]
    ZeroBasisPoints,
    #[error("recipient basis points sum must equal 10000")]
    BasisPointsDoNotSum,
    #[error("recipient token accounts must be distinct")]
    DuplicateRecipient,
    #[error("split allocation arithmetic overflowed")]
    ArithmeticOverflow,
    #[error("split allocation did not conserve the net amount")]
    ConservationViolation,
}

impl SplitPlan {
    pub fn new(
        merchant: String,
        mint: String,
        version: u64,
        recipients: Vec<SplitRecipient>,
    ) -> Result<Self, SplitPlanError> {
        parse_pubkey("merchant", &merchant)?;
        parse_pubkey("mint", &mint)?;
        if version == 0 {
            return Err(SplitPlanError::InvalidVersion);
        }
        if recipients.is_empty() || recipients.len() > MAX_RECIPIENTS {
            return Err(SplitPlanError::InvalidRecipientCount);
        }

        let mut total = 0_u32;
        let mut addresses = Vec::with_capacity(recipients.len());
        for recipient in &recipients {
            parse_pubkey("recipient", &recipient.token_account)?;
            if recipient.basis_points == 0 {
                return Err(SplitPlanError::ZeroBasisPoints);
            }
            total = total
                .checked_add(u32::from(recipient.basis_points))
                .ok_or(SplitPlanError::BasisPointsDoNotSum)?;
            if addresses
                .iter()
                .any(|address: &String| address == recipient.token_account.trim())
            {
                return Err(SplitPlanError::DuplicateRecipient);
            }
            addresses.push(recipient.token_account.trim().to_string());
        }
        if total != u32::from(BASIS_POINTS_DENOMINATOR) {
            return Err(SplitPlanError::BasisPointsDoNotSum);
        }

        Ok(Self {
            merchant: merchant.trim().to_string(),
            mint: mint.trim().to_string(),
            version,
            recipients,
        })
    }

    pub fn digest(&self) -> Result<[u8; 32], SplitPlanError> {
        let merchant = parse_pubkey("merchant", &self.merchant)?;
        let mint = parse_pubkey("mint", &self.mint)?;
        let mut input = Vec::with_capacity(128);
        input.extend_from_slice(SPLIT_PLAN_SCHEMA.as_bytes());
        input.push(0);
        input.extend_from_slice(merchant.as_ref());
        input.extend_from_slice(mint.as_ref());
        input.extend_from_slice(&self.version.to_le_bytes());
        input.push(
            u8::try_from(self.recipients.len())
                .map_err(|_| SplitPlanError::InvalidRecipientCount)?,
        );
        for recipient in &self.recipients {
            let address = parse_pubkey("recipient", &recipient.token_account)?;
            input.extend_from_slice(address.as_ref());
            input.extend_from_slice(&recipient.basis_points.to_le_bytes());
        }
        Ok(Sha256::digest(input).into())
    }

    pub fn allocate(&self, net_atomic: u64) -> Result<Vec<Allocation>, SplitPlanError> {
        let mut amounts = [0_u64; MAX_RECIPIENTS];
        let mut remainders = [0_u128; MAX_RECIPIENTS];
        let mut floor_sum = 0_u64;

        for (index, recipient) in self.recipients.iter().enumerate() {
            let numerator = u128::from(net_atomic)
                .checked_mul(u128::from(recipient.basis_points))
                .ok_or(SplitPlanError::ArithmeticOverflow)?;
            let base = numerator / u128::from(BASIS_POINTS_DENOMINATOR);
            amounts[index] = u64::try_from(base).map_err(|_| SplitPlanError::ArithmeticOverflow)?;
            remainders[index] = numerator % u128::from(BASIS_POINTS_DENOMINATOR);
            floor_sum = floor_sum
                .checked_add(amounts[index])
                .ok_or(SplitPlanError::ArithmeticOverflow)?;
        }

        let residual = net_atomic
            .checked_sub(floor_sum)
            .ok_or(SplitPlanError::ConservationViolation)?;
        let residual_count =
            usize::try_from(residual).map_err(|_| SplitPlanError::ConservationViolation)?;
        if residual_count > self.recipients.len() {
            return Err(SplitPlanError::ConservationViolation);
        }

        let mut selected = [false; MAX_RECIPIENTS];
        for _ in 0..residual_count {
            let mut best: Option<usize> = None;
            for index in 0..self.recipients.len() {
                if selected[index]
                    || best.is_some_and(|best_index| remainders[index] <= remainders[best_index])
                {
                    continue;
                }
                // Keeping the first index on an equal remainder is the explicit
                // ascending-ordinal tie rule.
                best = Some(index);
            }
            let index = best.ok_or(SplitPlanError::ConservationViolation)?;
            selected[index] = true;
            amounts[index] = amounts[index]
                .checked_add(1)
                .ok_or(SplitPlanError::ArithmeticOverflow)?;
        }

        let total = amounts[..self.recipients.len()]
            .iter()
            .try_fold(0_u64, |sum, amount| sum.checked_add(*amount))
            .ok_or(SplitPlanError::ArithmeticOverflow)?;
        if total != net_atomic {
            return Err(SplitPlanError::ConservationViolation);
        }

        Ok(self
            .recipients
            .iter()
            .zip(amounts)
            .map(|(recipient, amount_atomic)| Allocation {
                token_account: recipient.token_account.trim().to_string(),
                amount_atomic,
            })
            .collect())
    }
}

fn parse_pubkey(field: &'static str, value: &str) -> Result<Pubkey, SplitPlanError> {
    Pubkey::from_str(value.trim()).map_err(|_| SplitPlanError::InvalidPublicKey { field })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MERCHANT: &str = "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR";
    const MINT: &str = "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq";
    const RECIPIENT_A: &str = "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF";
    const RECIPIENT_B: &str = "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8";

    fn plan(a: u16, b: u16) -> SplitPlan {
        SplitPlan::new(
            MERCHANT.to_string(),
            MINT.to_string(),
            1,
            vec![
                SplitRecipient {
                    token_account: RECIPIENT_A.to_string(),
                    basis_points: a,
                },
                SplitRecipient {
                    token_account: RECIPIENT_B.to_string(),
                    basis_points: b,
                },
            ],
        )
        .expect("valid plan")
    }

    #[test]
    fn allocates_exactly_divisible_amounts() {
        let allocations = plan(7_000, 3_000).allocate(1_000).expect("allocation");
        assert_eq!(allocations[0].amount_atomic, 700);
        assert_eq!(allocations[1].amount_atomic, 300);
    }

    #[test]
    fn allocates_residual_by_largest_remainder_and_ordinal_tie() {
        let allocations = plan(3_333, 6_667).allocate(1).expect("allocation");
        assert_eq!(allocations[0].amount_atomic, 0);
        assert_eq!(allocations[1].amount_atomic, 1);

        let allocations = plan(5_000, 5_000).allocate(1).expect("allocation");
        assert_eq!(allocations[0].amount_atomic, 1);
        assert_eq!(allocations[1].amount_atomic, 0);
    }

    #[test]
    fn accepts_max_atomic_amount_without_float_rounding() {
        let allocations = plan(5_000, 5_000).allocate(u64::MAX).expect("allocation");
        assert_eq!(
            allocations[0].amount_atomic + allocations[1].amount_atomic,
            u64::MAX
        );
    }

    #[test]
    fn rejects_invalid_plan_shapes() {
        assert_eq!(
            SplitPlan::new(
                MERCHANT.to_string(),
                MINT.to_string(),
                0,
                vec![SplitRecipient {
                    token_account: RECIPIENT_A.to_string(),
                    basis_points: BASIS_POINTS_DENOMINATOR,
                }],
            )
            .expect_err("zero version must fail"),
            SplitPlanError::InvalidVersion
        );
        assert_eq!(
            SplitPlan::new(
                MERCHANT.to_string(),
                MINT.to_string(),
                1,
                vec![SplitRecipient {
                    token_account: RECIPIENT_A.to_string(),
                    basis_points: 9_999,
                }],
            )
            .expect_err("non-conserving bps must fail"),
            SplitPlanError::BasisPointsDoNotSum
        );
    }

    #[test]
    fn digest_is_stable_and_order_sensitive() {
        let first = plan(7_000, 3_000).digest().expect("digest");
        let second = plan(7_000, 3_000).digest().expect("digest");
        assert_eq!(first, second);
        assert_ne!(
            first,
            SplitPlan::new(
                MERCHANT.to_string(),
                MINT.to_string(),
                2,
                vec![
                    SplitRecipient {
                        token_account: RECIPIENT_A.to_string(),
                        basis_points: 7_000,
                    },
                    SplitRecipient {
                        token_account: RECIPIENT_B.to_string(),
                        basis_points: 3_000,
                    },
                ],
            )
            .expect("valid plan")
            .digest()
            .expect("digest")
        );
    }

    proptest! {
        #[test]
        fn allocation_always_conserves_for_bounded_two_way_plans(
            first_bps in 1_u16..BASIS_POINTS_DENOMINATOR,
            net_atomic in any::<u64>(),
        ) {
            let allocations = plan(first_bps, BASIS_POINTS_DENOMINATOR - first_bps)
                .allocate(net_atomic)
                .expect("bounded proportional plans must allocate");
            let total = allocations
                .iter()
                .try_fold(0_u64, |sum, allocation| sum.checked_add(allocation.amount_atomic))
                .expect("allocation sum cannot overflow");
            prop_assert_eq!(total, net_atomic);
        }
    }

    #[derive(Deserialize)]
    struct SplitPlanVector {
        schema: String,
        plan: SplitPlan,
        expected_digest: Vec<u8>,
        cases: Vec<SplitPlanVectorCase>,
    }

    #[derive(Deserialize)]
    struct SplitPlanVectorCase {
        net_atomic: String,
        allocations: Vec<String>,
    }

    #[test]
    fn matches_the_shared_split_plan_vector() {
        let vector: SplitPlanVector =
            serde_json::from_str(include_str!("../../../test-vectors/split-plan-v1.json"))
                .expect("split plan vector must be valid JSON");
        assert_eq!(vector.schema, SPLIT_PLAN_SCHEMA);
        assert_eq!(
            vector.expected_digest,
            vector.plan.digest().expect("vector plan digest")
        );
        for case in vector.cases {
            let actual = vector
                .plan
                .allocate(case.net_atomic.parse().expect("vector amount"))
                .expect("vector allocation");
            let expected = case
                .allocations
                .into_iter()
                .map(|amount| amount.parse::<u64>().expect("vector allocation amount"))
                .collect::<Vec<_>>();
            assert_eq!(
                actual
                    .into_iter()
                    .map(|allocation| allocation.amount_atomic)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}
