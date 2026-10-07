#![no_main]

use libfuzzer_sys::fuzz_target;
use texpose::NumericError;
use texpose_fuzz::{checked_result_matches, dim_from_fraction, ordering_matches, read_i128};

const INPUT_BYTES: usize = 64;

fuzz_target!(|data: &[u8]| {
    if data.len() < INPUT_BYTES {
        return;
    }

    let (lhs, lhs_ref) = dim_from_fraction(read_i128(data, 0), read_i128(data, 16));
    let (rhs, rhs_ref) = dim_from_fraction(read_i128(data, 32), read_i128(data, 48));

    checked_result_matches(lhs.checked_add(&rhs), &(lhs_ref.clone() + rhs_ref.clone()));
    checked_result_matches(lhs.checked_sub(&rhs), &(lhs_ref.clone() - rhs_ref.clone()));
    checked_result_matches(lhs.checked_mul(&rhs), &(lhs_ref.clone() * rhs_ref.clone()));
    ordering_matches(&lhs, &rhs, lhs_ref.cmp(&rhs_ref));

    if rhs.is_zero() {
        assert_eq!(lhs.checked_div(&rhs), Err(NumericError::ZeroDenominator));
    } else {
        checked_result_matches(lhs.checked_div(&rhs), &(lhs_ref / rhs_ref));
    }
});
