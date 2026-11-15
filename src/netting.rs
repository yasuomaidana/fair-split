//! Minimum-transfer balance netting for zero-sum party balances.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use crate::{Error, Result};

/// A directed pairwise settlement transfer from a debtor party to a creditor party.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transfer<P> {
    /// Party sending value (whose initial net balance was negative).
    pub from: P,
    /// Party receiving value (whose initial net balance was positive).
    pub to: P,
    /// Positive transfer magnitude.
    pub value: i64,
}

/// Computes a minimal set of pairwise [`Transfer`]s that settles zero-sum signed `balances`.
///
/// Duplicate party keys are summed before settlement. Positive balances represent net
/// creditors (to be paid) and negative balances represent net debtors (who pay). Uses
/// deterministic greedy largest-creditor/largest-debtor matching, guaranteeing at most
/// `n - 1` transfers for `n` non-zero parties.
///
/// # Errors
///
/// - [`Error::NonZeroSum`] if the balances do not sum to zero.
/// - [`Error::Overflow`] if balance aggregation or a required transfer exceeds `i64`/`i128`.
pub fn settle<P, I>(balances: I) -> Result<Vec<Transfer<P>>>
where
    P: Ord + Clone,
    I: IntoIterator<Item = (P, i64)>,
{
    let mut aggregated: BTreeMap<P, i128> = BTreeMap::new();
    for (party, balance) in balances {
        let entry = aggregated.entry(party).or_insert(0);
        *entry = entry.checked_add(balance as i128).ok_or(Error::Overflow)?;
    }
    settle_aggregated(aggregated)
}

/// Borrowed-entry variant of [`settle`] suitable for `&HashMap<P, i64>` or `&BTreeMap<P, i64>`.
///
/// # Errors
///
/// - [`Error::NonZeroSum`] if the balances do not sum to zero.
/// - [`Error::Overflow`] if balance aggregation or a required transfer exceeds `i64`/`i128`.
pub fn settle_refs<'a, P, I>(balances: I) -> Result<Vec<Transfer<P>>>
where
    P: Ord + Clone + 'a,
    I: IntoIterator<Item = (&'a P, &'a i64)>,
{
    settle(
        balances
            .into_iter()
            .map(|(party, &bal)| (party.clone(), bal)),
    )
}

fn settle_aggregated<P: Ord + Clone>(aggregated: BTreeMap<P, i128>) -> Result<Vec<Transfer<P>>> {
    let total = aggregated
        .values()
        .try_fold(0_i128, |acc, &v| acc.checked_add(v))
        .ok_or(Error::Overflow)?;
    if total != 0 {
        return Err(Error::NonZeroSum { sum: total });
    }

    let mut creditors: BinaryHeap<(i128, Reverse<P>)> = BinaryHeap::new();
    let mut debtors: BinaryHeap<(i128, Reverse<P>)> = BinaryHeap::new();

    for (party, balance) in aggregated {
        if balance > 0 {
            creditors.push((balance, Reverse(party)));
        } else if balance < 0 {
            debtors.push((-balance, Reverse(party)));
        }
    }

    let mut transfers = Vec::new();
    while let (Some((cred_val, Reverse(cred_p))), Some((debt_val, Reverse(debt_p)))) =
        (creditors.pop(), debtors.pop())
    {
        let transfer_val = cred_val.min(debt_val);
        transfers.push(Transfer {
            from: debt_p.clone(),
            to: cred_p.clone(),
            value: i64::try_from(transfer_val).map_err(|_| Error::Overflow)?,
        });
        if cred_val > transfer_val {
            creditors.push((cred_val - transfer_val, Reverse(cred_p)));
        }
        if debt_val > transfer_val {
            debtors.push((debt_val - transfer_val, Reverse(debt_p)));
        }
    }

    Ok(transfers)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;

    use super::*;

    #[test]
    fn settles_chain_and_cancels_zero_parties() {
        let transfers = settle([("a", -50), ("b", 0), ("c", 50)]);
        assert_eq!(
            transfers,
            Ok(vec![Transfer {
                from: "a",
                to: "c",
                value: 50,
            }])
        );
    }

    #[test]
    fn settles_borrowed_map_and_duplicate_keys() {
        let map = HashMap::from([(1_u32, 30), (2_u32, -10), (3_u32, -20)]);
        assert_eq!(
            settle_refs(&map),
            Ok(vec![
                Transfer {
                    from: 3,
                    to: 1,
                    value: 20,
                },
                Transfer {
                    from: 2,
                    to: 1,
                    value: 10,
                },
            ])
        );

        let dup = settle([("a", 25), ("a", 25), ("b", -50)]);
        assert_eq!(
            dup,
            Ok(vec![Transfer {
                from: "b",
                to: "a",
                value: 50,
            }])
        );
    }

    #[test]
    fn handles_i64_min_debtor_without_overflow() {
        let transfers = settle([("a", i64::MAX), ("b", 1), ("c", i64::MIN)]);
        assert_eq!(
            transfers,
            Ok(vec![
                Transfer {
                    from: "c",
                    to: "a",
                    value: i64::MAX,
                },
                Transfer {
                    from: "c",
                    to: "b",
                    value: 1,
                },
            ])
        );
    }

    #[rstest]
    #[case(&[("a", 10), ("b", -5)], 5)]
    #[case(&[("a", 1)], 1)]
    fn rejects_non_zero_balances(#[case] input: &[(&str, i64)], #[case] sum: i128) {
        assert_eq!(
            settle(input.iter().copied()),
            Err(Error::NonZeroSum { sum })
        );
    }
}
