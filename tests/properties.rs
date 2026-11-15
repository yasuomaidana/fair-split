//! Property-based tests for allocation conservation, half-even rounding, and balance netting.

use std::collections::BTreeMap;

use fair_split::{ResiduePolicy, allocate, check_zero_sum, netting, round_half_even};
use proptest::prelude::*;

fn nonzero_denom() -> impl Strategy<Value = i128> {
    prop_oneof![1_i128..=1_000_000, -1_000_000_i128..=-1]
}

fn allocation_case() -> impl Strategy<Value = (i64, Vec<i64>, usize)> {
    (
        -1_000_000_000_i64..=1_000_000_000_i64,
        prop::collection::vec(0_i64..=100_000, 1..=20),
    )
        .prop_filter("at least one positive weight", |(_, w)| {
            w.iter().any(|&x| x > 0)
        })
        .prop_flat_map(|(total, weights)| {
            let len = weights.len();
            (Just(total), Just(weights), 0..len)
        })
}

fn zero_sum_balances() -> impl Strategy<Value = Vec<(u16, i64)>> {
    prop::collection::vec(-1_000_000_i64..=1_000_000_i64, 1..=25).prop_map(|head| {
        let sum: i64 = head.iter().sum();
        let mut pairs: Vec<(u16, i64)> = head
            .into_iter()
            .enumerate()
            .map(|(i, bal)| (i as u16, bal))
            .collect();
        pairs.push((pairs.len() as u16, -sum));
        pairs
    })
}

proptest! {
    #[test]
    fn allocation_parts_sum_to_total((total, weights, idx) in allocation_case()) {
        let weight_sum: i128 = weights.iter().map(|&w| w as i128).sum();
        for policy in [ResiduePolicy::LargestWeight, ResiduePolicy::Index(idx)] {
            let parts = allocate(total, &weights, policy).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(parts.len(), weights.len());

            let part_sum: i128 = parts.iter().map(|&p| p as i128).sum();
            prop_assert_eq!(part_sum, total as i128);

            for (&part, &weight) in parts.iter().zip(&weights) {
                if weight == 0 {
                    prop_assert_eq!(part, 0);
                }
                // Quota rule: |part - total * weight / weight_sum| < 1
                let diff = (part as i128) * weight_sum - (total as i128) * (weight as i128);
                prop_assert!(diff.abs() < weight_sum);
            }
        }
    }

    #[test]
    fn rounding_is_half_even(num in -1_000_000_000_i128..=1_000_000_000_i128, den in nonzero_denom()) {
        let rounded = round_half_even(num, den).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let error_twice = ((rounded as i128) * den - num).abs() * 2;
        let abs_den = den.abs();

        // Distance to exact rational is at most 0.5.
        prop_assert!(error_twice <= abs_den);

        // Exact half ties always round to an even integer.
        if error_twice == abs_den {
            prop_assert_eq!(rounded % 2, 0);
        }
    }

    #[test]
    fn rounding_exact_halves_always_even(
        k in -1_000_000_i128..=1_000_000_i128,
        half_den in 1_i128..=10_000,
    ) {
        let den = half_den * 2;
        let num = k * den + half_den;
        let rounded = round_half_even(num, den).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(rounded % 2, 0);
        prop_assert!(((rounded as i128) - k).abs() <= 1);
    }

    #[test]
    fn netting_conserves_balances(balances in zero_sum_balances()) {
        let raw_values: Vec<i64> = balances.iter().map(|&(_, b)| b).collect();
        check_zero_sum(&raw_values).map_err(|e| TestCaseError::fail(e.to_string()))?;

        let transfers = netting::settle(balances.clone())
            .map_err(|e| TestCaseError::fail(e.to_string()))?;

        let mut net: BTreeMap<u16, i128> = BTreeMap::new();
        for (party, bal) in &balances {
            *net.entry(*party).or_insert(0) += *bal as i128;
        }
        let active_parties = net.values().filter(|&&b| b != 0).count();
        if active_parties == 0 {
            prop_assert!(transfers.is_empty());
        } else {
            prop_assert!(transfers.len() < active_parties);
        }

        for t in &transfers {
            prop_assert!(t.value > 0);
            prop_assert_ne!(t.from, t.to);
            *net.entry(t.from).or_insert(0) += t.value as i128;
            *net.entry(t.to).or_insert(0) -= t.value as i128;
        }

        for remaining in net.values() {
            prop_assert_eq!(*remaining, 0);
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn serde_roundtrip_and_validation() {
    use fair_split::{Error, Rate, Scale, Scaled, netting::Transfer};

    let scale = Scale::TEN_THOUSAND;
    let json = serde_json::to_string(&scale);
    assert_eq!(json.as_deref().ok(), Some("10000"));
    assert_eq!(serde_json::from_str::<Scale>("10000").ok(), Some(scale));
    assert!(serde_json::from_str::<Scale>("0").is_err());

    let rate = Rate::new(150);
    if let Ok(r) = rate {
        assert_eq!(serde_json::from_str::<Rate>("150").ok(), Some(r));
    }
    assert!(serde_json::from_str::<Rate>("-5").is_err());

    if let Ok(s) = Scaled::<100>::new(42) {
        assert_eq!(serde_json::from_str::<Scaled<100>>("42").ok(), Some(s));
    }
    assert!(serde_json::from_str::<Scaled<0>>("42").is_err());

    let transfer = Transfer {
        from: "a".to_owned(),
        to: "b".to_owned(),
        value: 10,
    };
    if let Ok(encoded) = serde_json::to_string(&transfer) {
        assert_eq!(
            serde_json::from_str::<Transfer<String>>(&encoded).ok(),
            Some(transfer)
        );
    }

    let err = Error::NonZeroSum { sum: 5 };
    if let Ok(encoded) = serde_json::to_string(&err) {
        assert_eq!(serde_json::from_str::<Error>(&encoded).ok(), Some(err));
    }
}
