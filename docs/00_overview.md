# Architecture & Mathematical Overview

`fair-split` provides exact fixed-point integer arithmetic, deterministic proportional allocation, conservation verification, and pairwise balance netting without floating-point representation or database dependencies.

## Design Invariants

1. **No Floating-Point Arithmetic:** All quantities, scales, ratios, rates, and balances are represented as signed integers (`i64` / `i32`).
2. **128-bit Intermediate Precision:** Multiplications, divisions, rescalings, and conservation sums are computed in `i128` (and `u128` for remainder comparisons) before converting back to `i64`.
3. **Panic-Free Library Code:** No `unwrap`, `expect`, or `panic` calls appear in library code. Out-of-range scales, zero divisors, negative weights, overflow, and non-zero conservation sums return explicit `fair_split::Error` variants.
4. **Determinism:** All operations depend solely on their inputs and produce identical results across platforms and runs.

## Module Breakdown

| Module | Responsibility | Key Types / Functions |
|---|---|---|
| `round` | Unbiased half-even (banker's) rounding integer division | `round_half_even` |
| `scale` | Configurable fixed-point scales, rescaling, and ratio multiplication | `Scale`, `Scaled<const SCALE: i64>`, `rescale`, `mul_ratio`, `scaled_mul`, `scaled_div` |
| `rate` | Non-negative rates in parts per ten thousand (`0..=1_000_000`) | `Rate`, `apply_rate`, `RATE_SCALE`, `MAX_RATE` |
| `allocate` | Largest-remainder proportional allocation with exact sum conservation | `allocate`, `ResiduePolicy` |
| `conserve` | Checked zero-sum conservation verification | `check_zero_sum` |
| `netting` | Minimal pairwise transfers settling zero-sum balances | `Transfer<P>`, `settle`, `settle_refs` |
| `error` | Mathematical error enum and `Result<T>` alias | `Error`, `Result` |

## Rounding & Allocation Guarantees

- **Half-Even Rounding:** For `round_half_even(numerator, denominator)`, ties where `2 * |remainder| == |denominator|` round toward the nearest even integer.
- **Exact Sum & Quota Rule:** For `allocate(total, weights, policy)` with `W = ∑ weights > 0`, the returned parts satisfy `∑ parts == total` and every part satisfies `|part_i - total * weight_i / W| < 1`. Any share with `weight_i == 0` receives `0`.
- **Balance Conservation in Netting:** For zero-sum signed balances across `n` non-zero parties, `netting::settle` produces at most `n - 1` strictly positive transfers (`from != to`) whose application reduces every party's net balance to `0`.
