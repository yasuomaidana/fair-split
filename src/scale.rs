//! Configurable scaled-integer arithmetic, rescaling, and ratio multiplication.

use core::fmt;

use crate::{Error, Rate, ResiduePolicy, Result, allocate, round_half_even};

/// A strictly positive integer scaling factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "i64", into = "i64"))]
pub struct Scale(i64);

impl Scale {
    /// Unit scale (`1`).
    pub const UNITY: Self = Self(1);
    /// Hundredths scale (`100` = `10^2`).
    pub const CENTI: Self = Self(100);
    /// Thousandths scale (`1_000` = `10^3`).
    pub const MILLI: Self = Self(1_000);
    /// Ten-thousandths scale (`10_000` = `10^4`).
    pub const TEN_THOUSAND: Self = Self(10_000);
    /// Millionths scale (`1_000_000` = `10^6`).
    pub const MICRO: Self = Self(1_000_000);
    /// Billionths scale (`1_000_000_000` = `10^9`).
    pub const NANO: Self = Self(1_000_000_000);
    /// High-precision ten-billionths scale (`10_000_000_000` = `10^10`).
    pub const E10: Self = Self(10_000_000_000);

    /// Creates a [`Scale`] from a strictly positive integer factor.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] if `scale <= 0`.
    pub const fn new(scale: i64) -> Result<Self> {
        if scale <= 0 {
            return Err(Error::InvalidScale { scale });
        }
        Ok(Self(scale))
    }

    /// Creates a power-of-ten [`Scale`] equal to `10^decimals`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Overflow`] if `10^decimals` exceeds `i64::MAX`.
    pub const fn from_decimals(decimals: u32) -> Result<Self> {
        match 10_i64.checked_pow(decimals) {
            Some(scale) => Ok(Self(scale)),
            None => Err(Error::Overflow),
        }
    }

    /// Returns the underlying positive scale factor.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for Scale {
    type Error = Error;

    fn try_from(scale: i64) -> Result<Self> {
        Self::new(scale)
    }
}

impl From<Scale> for i64 {
    fn from(scale: Scale) -> Self {
        scale.0
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Rescales `value` from scale `from` to scale `to` using `i128` intermediates and half-even rounding.
///
/// # Errors
///
/// Returns [`Error::Overflow`] if the rescaled result exceeds `i64`.
pub fn rescale(value: i64, from: Scale, to: Scale) -> Result<i64> {
    if from == to {
        return Ok(value);
    }
    let g = gcd_i64(from.get(), to.get());
    let num = (value as i128)
        .checked_mul((to.get() / g) as i128)
        .ok_or(Error::Overflow)?;
    let den = (from.get() / g) as i128;
    round_half_even(num, den)
}

/// Multiplies `lhs` by `rhs` at `scale` using `i128` intermediates and half-even rounding.
///
/// Computes `round_half_even(lhs * rhs, scale)`.
///
/// # Errors
///
/// Returns [`Error::Overflow`] if the rounded product exceeds `i64`.
pub fn scaled_mul(lhs: i64, rhs: i64, scale: Scale) -> Result<i64> {
    let product = (lhs as i128)
        .checked_mul(rhs as i128)
        .ok_or(Error::Overflow)?;
    round_half_even(product, scale.get() as i128)
}

/// Multiplies `value` by a scaled `ratio` (at `ratio_scale`), keeping `value`'s scale.
///
/// # Errors
///
/// Returns [`Error::Overflow`] if the rounded product exceeds `i64`.
pub fn mul_ratio(value: i64, ratio: i64, ratio_scale: Scale) -> Result<i64> {
    scaled_mul(value, ratio, ratio_scale)
}

/// Divides `lhs` by `rhs` at `scale` using `i128` intermediates and half-even rounding.
///
/// Computes `round_half_even(lhs * scale, rhs)`.
///
/// # Errors
///
/// - [`Error::DivisionByZero`] if `rhs == 0`.
/// - [`Error::Overflow`] if the rounded quotient exceeds `i64`.
pub fn scaled_div(lhs: i64, rhs: i64, scale: Scale) -> Result<i64> {
    let numerator = (lhs as i128)
        .checked_mul(scale.get() as i128)
        .ok_or(Error::Overflow)?;
    round_half_even(numerator, rhs as i128)
}

/// A fixed-point signed integer value with compile-time scale `SCALE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "i64", into = "i64"))]
pub struct Scaled<const SCALE: i64>(i64);

impl<const SCALE: i64> Scaled<SCALE> {
    /// Compile-time scale factor.
    pub const SCALE: i64 = SCALE;

    /// Wraps a raw scaled integer after validating `SCALE > 0`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] if `SCALE <= 0`.
    pub const fn new(value: i64) -> Result<Self> {
        if SCALE <= 0 {
            return Err(Error::InvalidScale { scale: SCALE });
        }
        Ok(Self(value))
    }

    /// Constructs a scaled value from an unscaled whole integer (`whole * SCALE`).
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidScale`] if `SCALE <= 0`.
    /// - [`Error::Overflow`] if `whole * SCALE` overflows `i64`.
    pub const fn from_whole(whole: i64) -> Result<Self> {
        if SCALE <= 0 {
            return Err(Error::InvalidScale { scale: SCALE });
        }
        match whole.checked_mul(SCALE) {
            Some(raw) => Ok(Self(raw)),
            None => Err(Error::Overflow),
        }
    }

    /// Returns the underlying scaled integer.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }

    /// Returns the validated [`Scale`] for `SCALE`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] if `SCALE <= 0`.
    pub const fn scale(self) -> Result<Scale> {
        Scale::new(SCALE)
    }

    /// Checked addition using an `i128` intermediate.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        i64::try_from((self.0 as i128) + (rhs.0 as i128))
            .map_err(|_| Error::Overflow)
            .and_then(Self::new)
    }

    /// Checked subtraction using an `i128` intermediate.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        i64::try_from((self.0 as i128) - (rhs.0 as i128))
            .map_err(|_| Error::Overflow)
            .and_then(Self::new)
    }

    /// Checked negation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn checked_neg(self) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        self.0
            .checked_neg()
            .ok_or(Error::Overflow)
            .and_then(Self::new)
    }

    /// Checked absolute value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn checked_abs(self) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        self.0
            .checked_abs()
            .ok_or(Error::Overflow)
            .and_then(Self::new)
    }

    /// Multiplies two values of the same scale using half-even rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        let scale = Scale::new(SCALE)?;
        scaled_mul(self.0, rhs.0, scale).and_then(Self::new)
    }

    /// Divides `self` by `rhs` at the same scale using half-even rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`], [`Error::DivisionByZero`], or [`Error::Overflow`].
    pub fn checked_div(self, rhs: Self) -> Result<Self> {
        let scale = Scale::new(SCALE)?;
        scaled_div(self.0, rhs.0, scale).and_then(Self::new)
    }

    /// Multiplies by an unscaled integer `factor`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn checked_mul_int(self, factor: i64) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        i64::try_from((self.0 as i128) * (factor as i128))
            .map_err(|_| Error::Overflow)
            .and_then(Self::new)
    }

    /// Divides by an unscaled integer `divisor` using half-even rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`], [`Error::DivisionByZero`], or [`Error::Overflow`].
    pub fn checked_div_int(self, divisor: i64) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        round_half_even(self.0 as i128, divisor as i128).and_then(Self::new)
    }

    /// Multiplies `self` by a scaled `ratio`, preserving `SCALE`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn mul_ratio<const RATIO_SCALE: i64>(self, ratio: Scaled<RATIO_SCALE>) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        let ratio_scale = Scale::new(RATIO_SCALE)?;
        mul_ratio(self.0, ratio.0, ratio_scale).and_then(Self::new)
    }

    /// Rescales `self` from `SCALE` to `TO_SCALE` using half-even rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn rescale<const TO_SCALE: i64>(self) -> Result<Scaled<TO_SCALE>> {
        let from = Scale::new(SCALE)?;
        let to = Scale::new(TO_SCALE)?;
        rescale(self.0, from, to).and_then(Scaled::new)
    }

    /// Applies a [`Rate`] (in parts per ten thousand) with half-even rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidScale`] or [`Error::Overflow`].
    pub fn apply_rate(self, rate: Rate) -> Result<Self> {
        let _ = Scale::new(SCALE)?;
        rate.apply(self.0).and_then(Self::new)
    }

    /// Splits `self` across `weights` using largest-remainder allocation.
    ///
    /// # Errors
    ///
    /// Propagates validation errors from [`allocate`] or [`Error::InvalidScale`].
    pub fn allocate(self, weights: &[i64], policy: ResiduePolicy) -> Result<Vec<Self>> {
        let _ = Scale::new(SCALE)?;
        allocate(self.0, weights, policy)?
            .into_iter()
            .map(Self::new)
            .collect()
    }
}

impl<const SCALE: i64> TryFrom<i64> for Scaled<SCALE> {
    type Error = Error;

    fn try_from(value: i64) -> Result<Self> {
        Self::new(value)
    }
}

impl<const SCALE: i64> From<Scaled<SCALE>> for i64 {
    fn from(scaled: Scaled<SCALE>) -> Self {
        scaled.0
    }
}

const fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(0)]
    #[case(-10)]
    fn rejects_non_positive_scale(#[case] raw: i64) {
        assert_eq!(
            Scale::try_from(raw),
            Err(Error::InvalidScale { scale: raw })
        );
        assert_eq!(Scaled::<0>::new(1), Err(Error::InvalidScale { scale: 0 }));
        assert_eq!(
            Scaled::<-5>::from_whole(1),
            Err(Error::InvalidScale { scale: -5 })
        );
    }

    #[test]
    fn scale_from_decimals_and_display() {
        let s = Scale::from_decimals(4);
        assert_eq!(s.map(Scale::get), Ok(10_000));
        assert_eq!(s.map(|v| v.to_string()), Ok("10000".to_owned()));
        assert_eq!(Scale::from_decimals(19), Err(Error::Overflow));
    }

    #[rstest]
    #[case(12_500, Scale::CENTI, Scale::TEN_THOUSAND, 1_250_000)]
    #[case(12_345, Scale::TEN_THOUSAND, Scale::CENTI, 123)]
    #[case(12_350, Scale::TEN_THOUSAND, Scale::CENTI, 124)]
    #[case(12_250, Scale::TEN_THOUSAND, Scale::CENTI, 122)]
    #[case(42, Scale::CENTI, Scale::CENTI, 42)]
    fn rescales_between_scales(
        #[case] value: i64,
        #[case] from: Scale,
        #[case] to: Scale,
        #[case] expected: i64,
    ) {
        assert_eq!(rescale(value, from, to), Ok(expected));
    }

    #[test]
    fn mul_ratio_and_scaled_ops() {
        // 100.0000 * 17.5000000000 = 1750.0000
        let val = Scaled::<10_000>::from_whole(100);
        let ratio = Scaled::<10_000_000_000>::new(175_000_000_000);
        if let (Ok(v), Ok(r)) = (val, ratio) {
            assert_eq!(v.mul_ratio(r).map(Scaled::value), Ok(17_500_000));
            assert_eq!(v.scale(), Ok(Scale::TEN_THOUSAND));
        }
        assert_eq!(
            mul_ratio(1_000_000, 175_000_000_000, Scale::E10),
            Ok(17_500_000)
        );
    }

    #[test]
    fn checked_arithmetic_and_allocation_on_scaled() {
        let a = Scaled::<100>::new(250);
        let b = Scaled::<100>::new(150);
        if let (Ok(a), Ok(b)) = (a, b) {
            assert_eq!(a.checked_add(b).map(Scaled::value), Ok(400));
            assert_eq!(a.checked_sub(b).map(Scaled::value), Ok(100));
            assert_eq!(a.checked_neg().map(Scaled::value), Ok(-250));
            assert_eq!(
                a.checked_neg()
                    .and_then(Scaled::checked_abs)
                    .map(Scaled::value),
                Ok(250)
            );
            assert_eq!(a.checked_mul(b).map(Scaled::value), Ok(375));
            assert_eq!(a.checked_div(b).map(Scaled::value), Ok(167));
            assert_eq!(a.checked_mul_int(3).map(Scaled::value), Ok(750));
            assert_eq!(a.checked_div_int(4).map(Scaled::value), Ok(62));
            assert_eq!(a.rescale::<10_000>().map(Scaled::value), Ok(25_000));
            assert_eq!(a.apply_rate(Rate::ONE).map(Scaled::value), Ok(250));
            assert_eq!(
                a.allocate(&[1, 1], ResiduePolicy::LargestWeight)
                    .map(|v| v.into_iter().map(i64::from).collect::<Vec<_>>()),
                Ok(vec![125, 125])
            );
        }
    }

    #[test]
    fn overflow_checks_on_scaled() {
        assert_eq!(Scaled::<100>::from_whole(i64::MAX), Err(Error::Overflow));
        if let Ok(max) = Scaled::<100>::new(i64::MAX) {
            assert_eq!(max.checked_add(max), Err(Error::Overflow));
            assert_eq!(max.checked_mul_int(2), Err(Error::Overflow));
        }
        if let Ok(min) = Scaled::<100>::new(i64::MIN) {
            assert_eq!(min.checked_neg(), Err(Error::Overflow));
            assert_eq!(min.checked_abs(), Err(Error::Overflow));
            assert_eq!(min.checked_sub(Scaled(1)), Err(Error::Overflow));
        }
        assert_eq!(scaled_div(1, 0, Scale::CENTI), Err(Error::DivisionByZero));
    }
}
