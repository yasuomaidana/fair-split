//! Half-even (banker's) rounding integer division.

use core::cmp::Ordering;

use crate::{Error, Result};

/// Divides `numerator` by `denominator` using round-half-to-even (banker's rounding).
///
/// Exact `.5` fractional ties round toward the nearest even integer.
///
/// # Errors
///
/// - [`Error::DivisionByZero`] if `denominator == 0`.
/// - [`Error::Overflow`] if the rounded quotient cannot fit in `i64` (including `i128::MIN / -1`).
pub fn round_half_even(numerator: i128, denominator: i128) -> Result<i64> {
    if denominator == 0 {
        return Err(Error::DivisionByZero);
    }

    let quotient = numerator.checked_div(denominator).ok_or(Error::Overflow)?;
    let rem = numerator.checked_rem(denominator).ok_or(Error::Overflow)?;
    let abs_rem = rem.unsigned_abs();
    let abs_den = denominator.unsigned_abs();

    // `abs_rem < abs_den <= 2^127`, so `abs_rem * 2 <= 2^128 - 2` never overflows `u128`.
    let round_away = match (abs_rem * 2).cmp(&abs_den) {
        Ordering::Greater => true,
        Ordering::Equal => quotient % 2 != 0,
        Ordering::Less => false,
    };

    let rounded = if round_away {
        let step = if (numerator >= 0) == (denominator >= 0) {
            1_i128
        } else {
            -1_i128
        };
        quotient.checked_add(step).ok_or(Error::Overflow)?
    } else {
        quotient
    };

    i64::try_from(rounded).map_err(|_| Error::Overflow)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(25, 10, 2)]
    #[case(35, 10, 4)]
    #[case(45, 10, 4)]
    #[case(55, 10, 6)]
    #[case(24, 10, 2)]
    #[case(26, 10, 3)]
    #[case(-25, 10, -2)]
    #[case(-35, 10, -4)]
    #[case(25, -10, -2)]
    #[case(35, -10, -4)]
    #[case(-35, -10, 4)]
    #[case(0, -5, 0)]
    #[case(i128::MIN, i128::MIN, 1)]
    #[case(i128::MAX, i128::MIN, -1)]
    #[case(1_i128 << 126, i128::MIN, 0)]
    fn rounds_half_to_even(#[case] num: i128, #[case] den: i128, #[case] expected: i64) {
        assert_eq!(round_half_even(num, den), Ok(expected));
    }

    #[rstest]
    #[case(10, 0, Error::DivisionByZero)]
    #[case(i128::MIN, -1, Error::Overflow)]
    #[case(i128::MAX, 1, Error::Overflow)]
    #[case(i128::MIN, 2, Error::Overflow)]
    fn rejects_invalid_division(#[case] num: i128, #[case] den: i128, #[case] expected: Error) {
        assert_eq!(round_half_even(num, den), Err(expected));
    }
}
