//! Non-negative rates expressed in parts per ten thousand (`10_000` = 1.0).

use crate::{Error, Result, round_half_even};

/// Scale denominator for rates in parts per ten thousand (`1` unit = `0.0001`).
pub const RATE_SCALE: i32 = 10_000;

/// Maximum accepted rate: `1_000_000` parts per ten thousand (`100.0x`).
pub const MAX_RATE: i32 = 1_000_000;

/// A validated non-negative rate expressed in parts per ten thousand.
///
/// For example, `150` represents `0.015` (`1.5%`), and `10_000` represents `1.0` (`100%`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "i32", into = "i32"))]
pub struct Rate(i32);

impl Rate {
    /// Zero rate (`0 / 10_000`).
    pub const ZERO: Self = Self(0);

    /// Unit rate (`10_000 / 10_000 = 1.0`).
    pub const ONE: Self = Self(RATE_SCALE);

    /// Constructs a [`Rate`] from parts per ten thousand.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRate`] if `value` is outside `0..=MAX_RATE`.
    pub const fn new(value: i32) -> Result<Self> {
        if value < 0 || value > MAX_RATE {
            return Err(Error::InvalidRate { value });
        }
        Ok(Self(value))
    }

    /// Returns the raw integer value in parts per ten thousand.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }

    /// Multiplies `value` by this rate using an `i128` intermediate and half-even rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Overflow`] if the rounded product exceeds `i64`.
    pub fn apply(self, value: i64) -> Result<i64> {
        let product = (value as i128)
            .checked_mul(self.0 as i128)
            .ok_or(Error::Overflow)?;
        round_half_even(product, RATE_SCALE as i128)
    }
}

impl TryFrom<i32> for Rate {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self> {
        Self::new(value)
    }
}

impl From<Rate> for i32 {
    fn from(rate: Rate) -> Self {
        rate.0
    }
}

/// Applies a rate expressed in parts per ten thousand to `value` with half-even rounding.
///
/// # Errors
///
/// - [`Error::InvalidRate`] if `rate` is outside `0..=MAX_RATE`.
/// - [`Error::Overflow`] if the rounded result exceeds `i64`.
pub fn apply_rate(value: i64, rate: i32) -> Result<i64> {
    Rate::new(rate)?.apply(value)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(-1)]
    #[case(MAX_RATE + 1)]
    fn rejects_out_of_range_rate(#[case] raw: i32) {
        assert_eq!(Rate::try_from(raw), Err(Error::InvalidRate { value: raw }));
    }

    #[rstest]
    #[case(1_000_000, 150, 15_000)]
    #[case(1, 5_000, 0)]
    #[case(3, 5_000, 2)]
    #[case(-3, 5_000, -2)]
    #[case(500, 0, 0)]
    #[case(500, RATE_SCALE, 500)]
    fn applies_rate_with_half_even_rounding(
        #[case] value: i64,
        #[case] rate: i32,
        #[case] expected: i64,
    ) {
        assert_eq!(apply_rate(value, rate), Ok(expected));
    }

    #[test]
    fn apply_reports_overflow() {
        let rate = Rate::try_from(MAX_RATE);
        assert_eq!(rate.map(Rate::get), Ok(MAX_RATE));
        assert_eq!(rate.map(i32::from), Ok(MAX_RATE));
        assert_eq!(apply_rate(i64::MAX, MAX_RATE), Err(Error::Overflow));
    }
}
