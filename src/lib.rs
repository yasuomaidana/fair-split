//! Dependency-light scaled-integer arithmetic, half-even rounding, largest-remainder
//! allocation, zero-sum conservation checks, and pairwise balance netting.
//!
//! All intermediate multiplications and sums use `i128` precision and return explicit
//! [`Result`] errors rather than panicking on overflow or invalid inputs.
//!
//! # Examples
//!
//! ```
//! use fair_split::{
//!     Rate, ResiduePolicy, Scale, Scaled, allocate, check_zero_sum, mul_ratio, netting,
//!     rescale, round_half_even,
//! };
//!
//! // Half-even rounding
//! assert_eq!(round_half_even(25, 10)?, 2);
//! assert_eq!(round_half_even(35, 10)?, 4);
//!
//! // Configurable scaled integers and rescaling
//! let val = Scaled::<10_000>::from_whole(100)?;
//! let rescaled: Scaled<100> = val.rescale()?;
//! assert_eq!(rescaled.value(), 10_000);
//! assert_eq!(rescale(12_350, Scale::TEN_THOUSAND, Scale::CENTI)?, 124);
//! assert_eq!(mul_ratio(1_000_000, 175_000_000_000, Scale::E10)?, 17_500_000);
//!
//! // Rate in parts per ten thousand (150 = 1.5%)
//! let fee = Rate::new(150)?.apply(1_000_000)?;
//! assert_eq!(fee, 15_000);
//!
//! // Largest-remainder allocation
//! let parts = allocate(100, &[1, 1, 1], ResiduePolicy::LargestWeight)?;
//! assert_eq!(parts, vec![34, 33, 33]);
//!
//! // Conservation check and pairwise netting
//! check_zero_sum([30, -10, -20])?;
//! let transfers = netting::settle([("alice", 30), ("bob", -10), ("carol", -20)])?;
//! assert_eq!(transfers.len(), 2);
//! # Ok::<(), fair_split::Error>(())
//! ```

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod allocate;
pub mod conserve;
pub mod error;
pub mod netting;
pub mod rate;
pub mod round;
pub mod scale;

pub use allocate::{ResiduePolicy, allocate};
pub use conserve::check_zero_sum;
pub use error::{Error, Result};
pub use rate::{MAX_RATE, RATE_SCALE, Rate, apply_rate};
pub use round::round_half_even;
pub use scale::{Scale, Scaled, mul_ratio, rescale, scaled_div, scaled_mul};
