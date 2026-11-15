# Changelog

All notable changes to `fair-split` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added
- `round_half_even`: checked `i128` division into `i64` using round-half-to-even tie-breaking.
- `Scale` and `Scaled<const SCALE: i64>` with `i128` intermediates, `rescale`, `scaled_mul`, `scaled_div`, and `mul_ratio`.
- `Rate` and `apply_rate` for non-negative rates expressed in parts per ten thousand (`0..=1_000_000`).
- `allocate` and `ResiduePolicy` (`LargestWeight`, `Index`) implementing deterministic largest-remainder proportional allocation.
- `check_zero_sum` for checked zero-sum conservation verification over signed `i64` sequences.
- `netting::settle`, `netting::settle_refs`, and `netting::Transfer<P>` for deterministic minimal pairwise settlement generic over party key `P: Ord + Clone`.
- Optional `serde` feature for serialization and validated deserialization.
- Property-based tests (`proptest`) and parameterized unit tests (`rstest`).
