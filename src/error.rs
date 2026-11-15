//! Error types for checked scaled-integer arithmetic, allocation, and netting.

use core::fmt;

/// Convenience result alias using [`enum@Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by checked arithmetic, allocation, conservation, and netting operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// Intermediate or final value exceeded the representable `i64` or `i128` range.
    Overflow,
    /// Divisor or total weight was zero.
    DivisionByZero,
    /// Scale factor must be strictly positive.
    InvalidScale {
        /// The rejected scale factor.
        scale: i64,
    },
    /// Rate in parts per ten thousand fell outside `0..=MAX_RATE`.
    InvalidRate {
        /// The rejected rate value.
        value: i32,
    },
    /// An allocation weight was negative.
    InvalidWeight {
        /// The rejected weight value.
        weight: i64,
    },
    /// An operation required at least one input item.
    EmptyInput,
    /// A residue target index exceeded the number of parts.
    IndexOutOfBounds {
        /// Requested target index.
        index: usize,
        /// Number of available parts.
        len: usize,
    },
    /// Signed values failed the zero-sum conservation check.
    NonZeroSum {
        /// Actual accumulated sum of the signed values.
        sum: i128,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => write!(f, "arithmetic overflow"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::InvalidScale { scale } => {
                write!(f, "scale must be strictly positive, got {scale}")
            }
            Self::InvalidRate { value } => {
                write!(f, "rate must be within 0..=1_000_000, got {value}")
            }
            Self::InvalidWeight { weight } => {
                write!(f, "allocation weight must be non-negative, got {weight}")
            }
            Self::EmptyInput => write!(f, "input sequence must not be empty"),
            Self::IndexOutOfBounds { index, len } => {
                write!(f, "index {index} out of bounds for length {len}")
            }
            Self::NonZeroSum { sum } => {
                write!(f, "conservation check failed: expected sum 0, got {sum}")
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Error::Overflow, "arithmetic overflow")]
    #[case(Error::DivisionByZero, "division by zero")]
    #[case(Error::InvalidScale { scale: 0 }, "scale must be strictly positive, got 0")]
    #[case(Error::InvalidRate { value: -1 }, "rate must be within 0..=1_000_000, got -1")]
    #[case(Error::InvalidWeight { weight: -2 }, "allocation weight must be non-negative, got -2")]
    #[case(Error::EmptyInput, "input sequence must not be empty")]
    #[case(Error::IndexOutOfBounds { index: 3, len: 2 }, "index 3 out of bounds for length 2")]
    #[case(Error::NonZeroSum { sum: 7 }, "conservation check failed: expected sum 0, got 7")]
    fn formats_all_error_variants(#[case] err: Error, #[case] expected: &str) {
        assert_eq!(err.to_string(), expected);
    }
}
