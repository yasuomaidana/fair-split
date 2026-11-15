//! Zero-sum conservation checks over signed integer values.

use core::borrow::Borrow;

use crate::{Error, Result};

/// Verifies that the signed `balances` sum exactly to zero.
///
/// Accumulation uses `i128` intermediates to prevent intermediate overflow.
///
/// # Errors
///
/// - [`Error::NonZeroSum`] if the sum of `balances` is non-zero.
/// - [`Error::Overflow`] if `i128` accumulation overflows.
pub fn check_zero_sum<I>(balances: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: Borrow<i64>,
{
    let sum = balances
        .into_iter()
        .try_fold(0_i128, |acc, balance| {
            acc.checked_add(*balance.borrow() as i128)
        })
        .ok_or(Error::Overflow)?;

    if sum == 0 {
        Ok(())
    } else {
        Err(Error::NonZeroSum { sum })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(&[])]
    #[case(&[0])]
    #[case(&[10, -10])]
    #[case(&[100, -40, -60])]
    #[case(&[i64::MAX, i64::MIN, 1])]
    fn accepts_zero_sum_slices(#[case] values: &[i64]) {
        assert_eq!(check_zero_sum(values), Ok(()));
    }

    #[rstest]
    #[case(&[1], 1)]
    #[case(&[10, -3], 7)]
    #[case(&[i64::MAX, i64::MAX], (i64::MAX as i128) * 2)]
    fn rejects_non_zero_sum_slices(#[case] values: &[i64], #[case] expected_sum: i128) {
        assert_eq!(
            check_zero_sum(values),
            Err(Error::NonZeroSum { sum: expected_sum })
        );
    }
}
