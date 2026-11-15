# fair-split

A small, dependency-light Rust crate for exact scaled-integer arithmetic, half-even (banker's) rounding, largest-remainder proportional allocation, zero-sum conservation checks, and minimal pairwise balance netting.

## Features

- **Zero required runtime dependencies** (`serde` support is optional via the `serde` feature).
- **No panics in library code**: all arithmetic uses `i128` intermediates and checked conversions returning `Result<T, fair_split::Error>`.
- **Configurable scale**: compile-time `Scaled<const SCALE: i64>` values and runtime `Scale` factors (`rescale`, `scaled_mul`, `scaled_div`, `mul_ratio`).
- **Half-even rounding**: `round_half_even` divides `i128` numerators by `i128` denominators with round-half-to-even tie-breaking.
- **Rates in parts per ten thousand**: `Rate` (`0..=1_000_000`, where `10_000` equals `1.0`) and `apply_rate`.
- **Largest-remainder allocation**: `allocate` splits a total proportionally across non-negative weights so that the resulting parts always sum exactly to the total, with a configurable `ResiduePolicy`.
- **Conservation checks**: `check_zero_sum` verifies signed sequences sum to zero in `i128` precision.
- **Pairwise balance netting**: `netting::settle` and `netting::settle_refs` compute a minimal set of directed `Transfer<P>` values over any `P: Ord + Clone` key type.

## Installation

```toml
[dependencies]
fair-split = "0.1"

# Optional serde support:
# fair-split = { version = "0.1", features = ["serde"] }
```

## Examples

### Scaled Arithmetic and Rescaling

```rust
use fair_split::{Scale, Scaled, mul_ratio, rescale, round_half_even};

fn main() -> fair_split::Result<()> {
    // Half-even (banker's) rounding
    assert_eq!(round_half_even(25, 10)?, 2);
    assert_eq!(round_half_even(35, 10)?, 4);

    // Compile-time scale (10^4)
    let quantity = Scaled::<10_000>::from_whole(100)?;
    let centi: Scaled<100> = quantity.rescale()?;
    assert_eq!(centi.value(), 10_000);

    // Runtime Scale rescaling and ratio multiplication
    assert_eq!(rescale(12_350, Scale::TEN_THOUSAND, Scale::CENTI)?, 124);
    assert_eq!(mul_ratio(1_000_000, 175_000_000_000, Scale::E10)?, 17_500_000);
    Ok(())
}
```

### Applying a Rate (Parts per Ten Thousand)

```rust
use fair_split::{Rate, apply_rate};

fn main() -> fair_split::Result<()> {
    // 150 parts per 10_000 = 1.5%
    let rate = Rate::new(150)?;
    assert_eq!(rate.apply(1_000_000)?, 15_000);
    assert_eq!(apply_rate(3, 5_000)?, 2); // 1.5 -> 2 (even)
    Ok(())
}
```

### Largest-Remainder Allocation

```rust
use fair_split::{ResiduePolicy, allocate};

fn main() -> fair_split::Result<()> {
    let parts = allocate(100, &[1, 1, 1], ResiduePolicy::LargestWeight)?;
    assert_eq!(parts, vec![34, 33, 33]);

    let chosen = allocate(100, &[1, 1, 1], ResiduePolicy::Index(2))?;
    assert_eq!(chosen, vec![33, 33, 34]);
    Ok(())
}
```

### Conservation Check and Pairwise Netting

```rust
use fair_split::{check_zero_sum, netting};

fn main() -> fair_split::Result<()> {
    let balances = [("alice", 30), ("bob", -10), ("carol", -20)];
    check_zero_sum(balances.iter().map(|(_, b)| b))?;

    let transfers = netting::settle(balances)?;
    assert_eq!(
        transfers,
        vec![
            netting::Transfer { from: "carol", to: "alice", value: 20 },
            netting::Transfer { from: "bob", to: "alice", value: 10 },
        ]
    );
    Ok(())
}
```

## Documentation

See [docs/00_overview.md](docs/00_overview.md) for the mathematical specification, precision guarantees, and module architecture.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
