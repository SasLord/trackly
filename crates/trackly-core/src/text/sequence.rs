//! First-free / max+1 / overflow computation over a set of taken numbers
//! (NUM-04/NUM-05).
//!
//! [`compute_next`] is a pure function: given the current set of numbers
//! already taken in a template's sequence and its digit width, it returns
//! where the next number should come from. It holds no state — callers
//! (Plan 03's repository, Plan 04's service) pass the live set of numbers
//! on every call, which is what makes soft-deletes, manual entry and CSV
//! import "just work" without a separate counter to keep in sync.

use crate::domain::number_templates::NextNumberResult;

/// Compute [`NextNumberResult`] for `taken` (the current set of numbers
/// already used in this template's sequence) and `digit_width` (`None` for
/// the unbounded `[X]` token, `Some(n)` for a fixed-width `[X…]` token of
/// `n` digits).
///
/// - `first_free` — the smallest `n >= 1` not present in `taken`.
/// - `max_plus_one` — `max(taken) + 1` (`1` when `taken` is empty).
/// - `has_gap` — `first_free != max_plus_one`, i.e. there is a hole lower
///   in the sequence than the running maximum.
/// - `max_plus_one_fits_width` — always `true` for an unbounded mask; for a
///   fixed width, whether `max_plus_one` itself still fits that width (it
///   can stop fitting while a lower gap remains free).
/// - `overflowed` — always `false` for an unbounded mask; for a fixed
///   width, `true` only when there is no free number left at all anywhere
///   in the width (i.e. even `first_free` does not fit).
pub fn compute_next(taken: &[u64], digit_width: Option<u32>) -> NextNumberResult {
    let mut sorted: Vec<u64> = taken.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut first_free: u64 = 1;
    for &n in &sorted {
        if n == first_free {
            first_free += 1;
        } else if n > first_free {
            break;
        }
    }

    let max_plus_one = sorted.last().copied().unwrap_or(0) + 1;
    let has_gap = first_free != max_plus_one;

    let (overflowed, max_plus_one_fits_width) = match digit_width {
        Some(width) => {
            let upper_bound = 10_u64.saturating_pow(width) - 1;
            (first_free > upper_bound, max_plus_one <= upper_bound)
        }
        None => (false, true),
    };

    NextNumberResult {
        first_free,
        max_plus_one,
        has_gap,
        max_plus_one_fits_width,
        overflowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_below_sixteen_taken_and_twenty_taken() {
        let taken: Vec<u64> = (1..=16).chain(std::iter::once(20)).collect();
        let r = compute_next(&taken, Some(6));
        assert_eq!(r.first_free, 17);
        assert_eq!(r.max_plus_one, 21);
        assert!(r.has_gap);
        assert!(!r.overflowed);
        assert!(r.max_plus_one_fits_width);
    }

    #[test]
    fn gap_from_five_hundred_to_five_ten() {
        let taken: Vec<u64> = (500..=510).collect();
        let r = compute_next(&taken, Some(6));
        assert_eq!(r.first_free, 1);
        assert_eq!(r.max_plus_one, 511);
        assert!(r.has_gap);
        assert!(!r.overflowed);
    }

    #[test]
    fn no_gap_when_taken_is_a_dense_prefix() {
        let r = compute_next(&[1, 2, 3], Some(6));
        assert_eq!(r.first_free, 4);
        assert_eq!(r.max_plus_one, 4);
        assert!(!r.has_gap);
        assert!(!r.overflowed);
    }

    #[test]
    fn empty_taken_set_starts_at_one() {
        let r = compute_next(&[], Some(6));
        assert_eq!(r.first_free, 1);
        assert_eq!(r.max_plus_one, 1);
        assert!(!r.has_gap);
        assert!(!r.overflowed);
    }

    #[test]
    fn gap_free_but_max_plus_one_does_not_fit_width() {
        let taken: Vec<u64> = (1..=16).chain(18..=99).collect();
        let r = compute_next(&taken, Some(2));
        assert_eq!(r.first_free, 17);
        assert!(r.has_gap);
        assert!(!r.overflowed, "a free number still exists (17)");
        assert!(
            !r.max_plus_one_fits_width,
            "max+1 (100) does not fit width 2"
        );
    }

    #[test]
    fn fully_occupied_width_is_overflowed() {
        let taken: Vec<u64> = (1..=99).collect();
        let r = compute_next(&taken, Some(2));
        assert!(r.overflowed, "no free number left in width 2");
    }

    #[test]
    fn unbounded_width_never_overflows_regardless_of_taken_set() {
        let taken: Vec<u64> = (1..=1_000).collect();
        let r = compute_next(&taken, None);
        assert!(!r.overflowed);
        assert!(r.max_plus_one_fits_width);
        assert_eq!(r.first_free, 1_001);
        assert_eq!(r.max_plus_one, 1_001);
    }

    #[test]
    fn pure_function_recomputes_after_simulated_soft_delete() {
        // Simulates "…000002" existing, then being soft-deleted: the
        // function holds no state, so calling it again with the number
        // removed from `taken` is enough to prove first_free updates.
        let before_delete: Vec<u64> = vec![1, 2, 3];
        let r_before = compute_next(&before_delete, Some(6));
        assert_eq!(r_before.first_free, 4);

        let after_delete: Vec<u64> = vec![1, 3];
        let r_after = compute_next(&after_delete, Some(6));
        assert_eq!(r_after.first_free, 2);
    }
}
