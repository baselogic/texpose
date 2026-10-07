use core::cmp::Ordering;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use texpose::{Dim, NumericError};

const I128_MAX_U: u128 = i128::MAX as u128;

pub fn assert_canonical(value: &Dim) {
    let (num, den) = value.as_ratio();
    assert!(den > 0, "Dim denominator must stay positive");
    if num == 0 {
        assert_eq!(den, 1, "Dim zero must stay canonical");
    }

    let normalized = BigRational::new(BigInt::from(num), BigInt::from(den));
    assert_eq!(normalized.numer(), &BigInt::from(num));
    assert_eq!(normalized.denom(), &BigInt::from(den));
}

pub fn dim_ratio(value: &Dim) -> BigRational {
    let (num, den) = value.as_ratio();
    BigRational::new(BigInt::from(num), BigInt::from(den))
}

pub fn assert_matches(value: &Dim, expected: &BigRational) {
    assert_canonical(value);
    assert_eq!(&dim_ratio(value), expected);
}

pub fn fits_dim(value: &BigRational) -> bool {
    let max = BigInt::from(i128::MAX);
    value.numer().abs() <= max && value.denom() <= &max
}

pub fn physical_layout_prelude_fits(root_em_size_pt: &BigRational) -> bool {
    let null_delimiter_space =
        BigRational::new(BigInt::from(6), BigInt::from(5)) / root_em_size_pt.clone();
    let delimiter_shortfall = BigRational::from_integer(BigInt::from(5)) / root_em_size_pt.clone();

    fits_dim(&null_delimiter_space) && fits_dim(&delimiter_shortfall)
}

pub fn checked_result_matches(actual: Result<Dim, NumericError>, expected: &BigRational) {
    if fits_dim(expected) {
        let actual = actual.expect("representable exact result must succeed");
        assert_matches(&actual, expected);
    } else {
        assert_eq!(actual, Err(NumericError::ArithmeticOverflow));
    }
}

pub fn bounded_signed(raw: i128) -> i128 {
    if raw == i128::MIN {
        -i128::MAX
    } else {
        raw
    }
}

pub fn positive_nonzero(raw: i128) -> i128 {
    let magnitude = raw.unsigned_abs().min(I128_MAX_U);
    if magnitude == 0 {
        1
    } else {
        magnitude as i128
    }
}

pub fn dim_from_fraction(num_raw: i128, den_raw: i128) -> (Dim, BigRational) {
    let num = bounded_signed(num_raw);
    let den = positive_nonzero(den_raw);
    let numerator = Dim::parse(&num.to_string()).expect("bounded i128 numerator");
    let denominator = Dim::parse(&den.to_string()).expect("positive i128 denominator");
    let value = numerator
        .checked_div(&denominator)
        .expect("canonical input fraction must fit Dim");
    let expected = BigRational::new(BigInt::from(num), BigInt::from(den));
    assert_matches(&value, &expected);
    (value, expected)
}

pub fn read_i128(data: &[u8], offset: usize) -> i128 {
    let bytes: [u8; 16] = data[offset..offset + 16]
        .try_into()
        .expect("fixed-width fuzz input");
    i128::from_le_bytes(bytes)
}

pub fn read_i64(data: &[u8], offset: usize) -> i64 {
    let bytes: [u8; 8] = data[offset..offset + 8]
        .try_into()
        .expect("fixed-width fuzz input");
    i64::from_le_bytes(bytes)
}

pub fn read_u16(data: &[u8], offset: usize) -> u16 {
    let bytes: [u8; 2] = data[offset..offset + 2]
        .try_into()
        .expect("fixed-width fuzz input");
    u16::from_le_bytes(bytes)
}

pub fn reference_decimal(input: &str) -> Option<BigRational> {
    let text = input.trim();
    if text.is_empty() {
        return None;
    }

    let (negative, unsigned) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    if unsigned.is_empty() {
        return None;
    }

    let mut exponent_split = unsigned.split(['e', 'E']);
    let mantissa = exponent_split.next()?;
    let exponent_text = exponent_split.next();
    if exponent_split.next().is_some() {
        return None;
    }

    let mut point_split = mantissa.split('.');
    let integer = point_split.next().unwrap_or_default();
    let fraction = point_split.next().unwrap_or_default();
    if point_split.next().is_some()
        || (integer.is_empty() && fraction.is_empty())
        || !integer.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }

    if let Some(raw) = exponent_text {
        if !valid_exponent_syntax(raw) {
            return None;
        }
    }

    if integer
        .bytes()
        .chain(fraction.bytes())
        .all(|digit| digit == b'0')
    {
        return Some(BigRational::zero());
    }

    let exponent = match exponent_text {
        Some(raw) => raw.parse::<i64>().ok()?,
        None => 0,
    };

    let mut digits = String::with_capacity(integer.len() + fraction.len());
    digits.push_str(integer);
    digits.push_str(fraction);
    let mut numerator = BigInt::parse_bytes(digits.as_bytes(), 10)?;
    if negative {
        numerator = -numerator;
    }

    let fraction_len = i128::try_from(fraction.len()).ok()?;
    let scale = fraction_len.checked_sub(i128::from(exponent))?;
    if scale >= 0 {
        let denominator = pow10(scale)?;
        Some(BigRational::new(numerator, denominator))
    } else {
        let factor = pow10(scale.checked_neg()?)?;
        Some(BigRational::from_integer(numerator * factor))
    }
}

fn valid_exponent_syntax(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    let digits = match raw.as_bytes()[0] {
        b'+' | b'-' => &raw[1..],
        _ => raw,
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn pow10(exponent: i128) -> Option<BigInt> {
    let exponent = u32::try_from(exponent).ok()?;
    Some(BigInt::from(10u8).pow(exponent))
}

pub fn ordering_matches(lhs: &Dim, rhs: &Dim, expected: Ordering) {
    assert_eq!(lhs.cmp(rhs), expected);
    assert_eq!(lhs.partial_cmp(rhs), Some(expected));
}

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use texpose::Dim;

    use super::{assert_matches, fits_dim, physical_layout_prelude_fits, reference_decimal};

    #[test]
    fn decimal_reference_covers_dim_syntax_examples() {
        for (text, expected) in [
            (".5", BigRational::new(BigInt::from(1), BigInt::from(2))),
            (
                "-12.50",
                BigRational::new(BigInt::from(-25), BigInt::from(2)),
            ),
            ("1.25e2", BigRational::from_integer(BigInt::from(125))),
            ("125e-2", BigRational::new(BigInt::from(5), BigInt::from(4))),
            (
                "0e999999999999999999999",
                BigRational::from_integer(BigInt::from(0)),
            ),
        ] {
            assert_eq!(reference_decimal(text), Some(expected.clone()));
            assert_matches(&Dim::parse(text).expect("Dim example"), &expected);
        }
    }

    #[test]
    fn dim_range_test_matches_normalized_ratio_limits() {
        let max = BigInt::from(i128::MAX);
        assert!(fits_dim(&BigRational::new(max.clone(), max.clone())));
        assert!(!fits_dim(
            &BigRational::from_integer(max + BigInt::from(1),)
        ));
    }

    #[test]
    fn physical_layout_prelude_tracks_internal_physical_constants() {
        let root = BigRational::from_integer(BigInt::from(i128::MAX));
        let requested = BigRational::from_integer(BigInt::from(1)) / root.clone();
        let null_delimiter_space =
            BigRational::new(BigInt::from(6), BigInt::from(5)) / root.clone();
        let delimiter_shortfall = BigRational::from_integer(BigInt::from(5)) / root.clone();

        assert!(fits_dim(&requested));
        assert!(!fits_dim(&null_delimiter_space));
        assert!(fits_dim(&delimiter_shortfall));
        assert!(!physical_layout_prelude_fits(&root));
    }
}
