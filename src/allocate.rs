//! Deterministic largest-remainder proportional allocation.

use core::cmp::Ordering;

use crate::{Error, Result};

/// Tie-breaking policy for assigning rounding remainder units across shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResiduePolicy {
    /// Break remainder ties by largest weight, then lowest index.
    #[default]
    LargestWeight,
    /// Break remainder ties in cyclic index order starting at `index`.
    Index(usize),
}

/// Splits `total` proportionally across non-negative `weights` so that `∑ parts == total`.
///
/// Uses largest-remainder (Hamilton) allocation in `i128` precision: each part receives
/// `trunc(total * weight / ∑weights)`, and the remaining `|total - ∑parts|` units are
/// distributed one unit at a time to the shares with the largest remainders. Ties are
/// resolved according to `policy`.
///
/// # Errors
///
/// - [`Error::EmptyInput`] if `weights` is empty.
/// - [`Error::InvalidWeight`] if any weight is negative.
/// - [`Error::IndexOutOfBounds`] if `policy` is [`ResiduePolicy::Index`] with `index >= weights.len()`.
/// - [`Error::DivisionByZero`] if all weights are zero.
/// - [`Error::Overflow`] if an intermediate or final value exceeds `i64`.
pub fn allocate(total: i64, weights: &[i64], policy: ResiduePolicy) -> Result<Vec<i64>> {
    let weight_sum = validate_inputs(weights, policy)?;
    let (mut parts, remainders, allocated) = initial_quotients(total, weights, weight_sum)?;

    let delta = (total as i128) - allocated;
    let remaining = usize::try_from(delta.unsigned_abs()).map_err(|_| Error::Overflow)?;
    if remaining > 0 {
        let step: i64 = if delta >= 0 { 1 } else { -1 };
        let mut order: Vec<usize> = (0..weights.len()).collect();
        order.sort_by(|&a, &b| {
            remainders[b]
                .cmp(&remainders[a])
                .then_with(|| tie_cmp(a, b, weights, policy))
        });
        for &idx in order.iter().take(remaining) {
            parts[idx] = parts[idx].checked_add(step).ok_or(Error::Overflow)?;
        }
    }

    Ok(parts)
}

fn validate_inputs(weights: &[i64], policy: ResiduePolicy) -> Result<i128> {
    if weights.is_empty() {
        return Err(Error::EmptyInput);
    }
    if let Some(&weight) = weights.iter().find(|&&w| w < 0) {
        return Err(Error::InvalidWeight { weight });
    }
    if let ResiduePolicy::Index(index) = policy
        && index >= weights.len()
    {
        return Err(Error::IndexOutOfBounds {
            index,
            len: weights.len(),
        });
    }
    let weight_sum: i128 = weights.iter().map(|&w| w as i128).sum();
    if weight_sum == 0 {
        return Err(Error::DivisionByZero);
    }
    Ok(weight_sum)
}

fn initial_quotients(
    total: i64,
    weights: &[i64],
    weight_sum: i128,
) -> Result<(Vec<i64>, Vec<u128>, i128)> {
    let mut parts = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    let mut allocated: i128 = 0;

    for &weight in weights {
        let product = (total as i128) * (weight as i128);
        let quotient = product / weight_sum;
        let rem = (product % weight_sum).unsigned_abs();
        parts.push(i64::try_from(quotient).map_err(|_| Error::Overflow)?);
        remainders.push(rem);
        allocated += quotient;
    }

    Ok((parts, remainders, allocated))
}

fn tie_cmp(a: usize, b: usize, weights: &[i64], policy: ResiduePolicy) -> Ordering {
    match policy {
        ResiduePolicy::LargestWeight => weights[b].cmp(&weights[a]).then_with(|| a.cmp(&b)),
        ResiduePolicy::Index(start) => {
            let len = weights.len();
            let dist_a = (a + len - start) % len;
            let dist_b = (b + len - start) % len;
            dist_a.cmp(&dist_b)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(100, &[1, 1, 1], ResiduePolicy::LargestWeight, vec![34, 33, 33])]
    #[case(100, &[1, 1, 1], ResiduePolicy::Index(2), vec![33, 33, 34])]
    #[case(10, &[1, 1, 1, 1, 1, 1, 1], ResiduePolicy::LargestWeight, vec![2, 2, 2, 1, 1, 1, 1])]
    #[case(10, &[1, 1, 1, 1, 1, 1, 1], ResiduePolicy::Index(5), vec![2, 1, 1, 1, 1, 2, 2])]
    #[case(-10, &[1, 1, 1], ResiduePolicy::LargestWeight, vec![-4, -3, -3])]
    #[case(7, &[0, 9, 0], ResiduePolicy::Index(0), vec![0, 7, 0])]
    fn splits_with_largest_remainder_and_policy(
        #[case] total: i64,
        #[case] weights: &[i64],
        #[case] policy: ResiduePolicy,
        #[case] expected: Vec<i64>,
    ) {
        assert_eq!(allocate(total, weights, policy), Ok(expected));
    }

    #[rstest]
    #[case(&[], ResiduePolicy::LargestWeight, Error::EmptyInput)]
    #[case(&[-1, 2], ResiduePolicy::LargestWeight, Error::InvalidWeight { weight: -1 })]
    #[case(&[0, 0], ResiduePolicy::LargestWeight, Error::DivisionByZero)]
    #[case(&[1, 1], ResiduePolicy::Index(2), Error::IndexOutOfBounds { index: 2, len: 2 })]
    fn rejects_invalid_inputs(
        #[case] weights: &[i64],
        #[case] policy: ResiduePolicy,
        #[case] expected: Error,
    ) {
        assert_eq!(allocate(10, weights, policy), Err(expected));
    }

    #[test]
    fn extreme_totals_still_sum_exactly() {
        let parts = allocate(i64::MAX, &[i64::MAX, i64::MAX], ResiduePolicy::Index(0));
        assert_eq!(parts.as_ref().map(Vec::len), Ok(2));
        if let Ok(parts) = parts {
            let sum: i128 = parts.iter().map(|&p| p as i128).sum();
            assert_eq!(sum, i64::MAX as i128);
        }
    }
}
