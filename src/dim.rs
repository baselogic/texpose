//! Layout dimension: exact, finite rational (`num / den`).
//!
//! TeX layout is ratios of integers (font units, mu = 1/18 em, style scales).
//! Hardware `f32` / `f64` never appear as calculation terminals.

use core::cmp::Ordering;
use core::fmt;
use core::num::{NonZeroU128, NonZeroU16};
use core::ops::Neg;

use crate::error::NumericError;

/// Kept in the public API. Layout values are exact rationals, not rounded bits.
pub const DIM_PREC: usize = 256;

const I128_MAX_U: u128 = i128::MAX as u128;

fn gcd_u(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Unsigned 256-bit scratch used only when the fast `i128` add/sub path
/// overflows before the canonical result can be reduced. This is deliberately
/// private and minimal: `Dim` itself remains an `i128/i128` rational.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Wide {
    hi: u128,
    lo: u128,
}

impl Wide {
    const fn zero() -> Self {
        Self { hi: 0, lo: 0 }
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        let (lo, carry) = self.lo.overflowing_add(other.lo);
        let hi = self
            .hi
            .checked_add(other.hi)?
            .checked_add(if carry { 1 } else { 0 })?;
        Some(Self { hi, lo })
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        if self < other {
            return None;
        }
        let (lo, borrow) = self.lo.overflowing_sub(other.lo);
        let hi = self
            .hi
            .checked_sub(other.hi)?
            .checked_sub(if borrow { 1 } else { 0 })?;
        Some(Self { hi, lo })
    }

    fn checked_shl1(self) -> Option<Self> {
        if self.hi >> 127 != 0 {
            return None;
        }
        Some(Self {
            hi: (self.hi << 1) | (self.lo >> 127),
            lo: self.lo << 1,
        })
    }

    fn bit(self, index: usize) -> u128 {
        if index < 128 {
            (self.lo >> index) & 1
        } else {
            (self.hi >> (index - 128)) & 1
        }
    }

    fn rem_u128(self, divisor: NonZeroU128) -> u128 {
        let divisor = divisor.get();
        let mut rem = 0u128;
        for index in (0..256).rev() {
            // `divisor <= i128::MAX`, so `rem < divisor < 2^127` before
            // this shift. The one-bit extension therefore fits in `u128`.
            rem = (rem << 1) | self.bit(index);
            if rem >= divisor {
                rem -= divisor;
            }
        }
        rem
    }

    fn div_exact_to_u128(self, divisor: NonZeroU128) -> Option<u128> {
        let divisor = divisor.get();
        let mut rem = 0u128;
        let mut quotient = 0u128;
        for index in (0..256).rev() {
            // Same bounded remainder invariant as `rem_u128`.
            rem = (rem << 1) | self.bit(index);
            if rem < divisor {
                continue;
            }
            rem -= divisor;
            if index >= 128 {
                return None;
            }
            quotient |= 1u128 << index;
        }
        if rem == 0 {
            Some(quotient)
        } else {
            None
        }
    }
}

fn wide_mul_u128(mut lhs: u128, mut rhs: u128) -> Option<Wide> {
    if lhs < rhs {
        core::mem::swap(&mut lhs, &mut rhs);
    }
    let mut result = Wide::zero();
    let mut term = Wide { hi: 0, lo: lhs };
    while rhs != 0 {
        if rhs & 1 != 0 {
            result = result.checked_add(term)?;
        }
        rhs >>= 1;
        if rhs != 0 {
            term = term.checked_shl1()?;
        }
    }
    Some(result)
}

fn reverse(ordering: Ordering) -> Ordering {
    match ordering {
        Ordering::Less => Ordering::Greater,
        Ordering::Equal => Ordering::Equal,
        Ordering::Greater => Ordering::Less,
    }
}

/// Compare two non-negative rationals without cross multiplication.
fn cmp_unsigned_rational(
    mut lhs_num: u128,
    mut lhs_den: u128,
    mut rhs_num: u128,
    mut rhs_den: u128,
) -> Ordering {
    let mut inverted = false;
    loop {
        let lhs_q = lhs_num / lhs_den;
        let rhs_q = rhs_num / rhs_den;
        if lhs_q != rhs_q {
            let order = lhs_q.cmp(&rhs_q);
            return if inverted { reverse(order) } else { order };
        }

        let lhs_r = lhs_num % lhs_den;
        let rhs_r = rhs_num % rhs_den;
        let order = match (lhs_r == 0, rhs_r == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => {
                lhs_num = lhs_den;
                lhs_den = lhs_r;
                rhs_num = rhs_den;
                rhs_den = rhs_r;
                inverted = !inverted;
                continue;
            }
        };
        return if inverted { reverse(order) } else { order };
    }
}

/// TeX-style dimension: width, height, depth, italic correction, mu.
///
/// A `Dim` is always finite and canonical: the denominator is positive, the
/// fraction is in lowest terms, and zero is represented only as `0/1`.
/// `i128::MIN` is excluded from the normalized numerator so negation is total.
///
/// One unit is one em at the current math style unless a method says otherwise.
///
/// # Examples
///
/// ```
/// use texpose::Dim;
///
/// let half = Dim::ratio(1, 2).unwrap();
/// assert_eq!(half, Dim::one().checked_div(&Dim::from_i64(2)).unwrap());
/// assert_ne!(Dim::zero(), Dim::one());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dim {
    num: i128,
    den: i128,
}

impl Dim {
    fn from_magnitudes(
        negative: bool,
        mut num: u128,
        mut den: u128,
        range_error: NumericError,
    ) -> Result<Self, NumericError> {
        if den == 0 {
            return Err(NumericError::ZeroDenominator);
        }
        if num == 0 {
            return Ok(Self::zero());
        }
        let g = gcd_u(num, den);
        num /= g;
        den /= g;
        if num > I128_MAX_U || den > I128_MAX_U {
            return Err(range_error);
        }
        let num = num as i128;
        Ok(Self {
            num: if negative { -num } else { num },
            den: den as i128,
        })
    }

    fn from_parts(num: i128, den: i128) -> Result<Self, NumericError> {
        if den == 0 {
            return Err(NumericError::ZeroDenominator);
        }
        let negative = (num < 0) ^ (den < 0);
        Self::from_magnitudes(
            negative,
            num.unsigned_abs(),
            den.unsigned_abs(),
            NumericError::OutOfRange,
        )
    }

    fn from_arithmetic_parts(num: i128, den: i128) -> Result<Self, NumericError> {
        if den == 0 {
            return Err(NumericError::ZeroDenominator);
        }
        let negative = (num < 0) ^ (den < 0);
        Self::from_magnitudes(
            negative,
            num.unsigned_abs(),
            den.unsigned_abs(),
            NumericError::ArithmeticOverflow,
        )
    }

    /// Zero em.
    #[must_use]
    pub const fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    /// One em.
    #[must_use]
    pub const fn one() -> Self {
        Self { num: 1, den: 1 }
    }

    /// Integer em count.
    #[must_use]
    pub fn from_i64(v: i64) -> Self {
        Self {
            num: i128::from(v),
            den: 1,
        }
    }

    /// Exact rational `num / den` em.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ZeroDenominator`] for `den == 0`.
    pub fn ratio(num: i64, den: i64) -> Result<Self, NumericError> {
        Self::from_parts(i128::from(num), i128::from(den))
    }

    /// Parse a decimal string with an optional sign, fraction, and `e` exponent.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::InvalidDecimal`] for malformed syntax,
    /// [`NumericError::ConversionOverflow`] when decimal digits cannot be
    /// accumulated exactly, and [`NumericError::OutOfRange`] when the exact
    /// rational does not fit the `Dim` contract.
    pub fn parse(s: &str) -> Result<Self, NumericError> {
        let text = s.trim();
        if text.is_empty() {
            return Err(NumericError::InvalidDecimal);
        }

        let (negative, unsigned) = match text.as_bytes()[0] {
            b'-' => (true, &text[1..]),
            b'+' => (false, &text[1..]),
            _ => (false, text),
        };
        if unsigned.is_empty() {
            return Err(NumericError::InvalidDecimal);
        }

        let mut exponent_split = unsigned.split(['e', 'E']);
        let mantissa = exponent_split.next().unwrap_or_default();
        let exponent_text = exponent_split.next();
        if exponent_split.next().is_some() {
            return Err(NumericError::InvalidDecimal);
        }

        let mut point_split = mantissa.split('.');
        let integer = point_split.next().unwrap_or_default();
        let fraction = point_split.next().unwrap_or_default();
        if point_split.next().is_some()
            || (integer.is_empty() && fraction.is_empty())
            || !integer.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(NumericError::InvalidDecimal);
        }

        if let Some(raw) = exponent_text {
            validate_decimal_exponent(raw)?;
        }

        let fraction_len =
            i64::try_from(fraction.len()).map_err(|_| NumericError::ConversionOverflow)?;
        let integer = integer.trim_start_matches('0');
        let trimmed_fraction = fraction.trim_end_matches('0');
        let (integer, fraction, trailing_zeros) = if trimmed_fraction.is_empty() {
            let trimmed_integer = integer.trim_end_matches('0');
            if trimmed_integer.is_empty() {
                return Ok(Self::zero());
            }
            let trailing = fraction
                .len()
                .checked_add(integer.len() - trimmed_integer.len())
                .ok_or(NumericError::ConversionOverflow)?;
            (trimmed_integer, "", trailing)
        } else {
            (
                integer,
                trimmed_fraction,
                fraction.len() - trimmed_fraction.len(),
            )
        };

        let mut magnitude = 0u128;
        for b in integer.bytes().chain(fraction.bytes()) {
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|n| n.checked_add(u128::from(b - b'0')))
                .ok_or(NumericError::ConversionOverflow)?;
        }

        let exponent = match exponent_text {
            None => 0i64,
            Some(raw) => parse_decimal_exponent(raw)?,
        };
        let trailing_zeros =
            i64::try_from(trailing_zeros).map_err(|_| NumericError::ConversionOverflow)?;
        let scale = fraction_len
            .checked_sub(exponent)
            .and_then(|value| value.checked_sub(trailing_zeros))
            .ok_or(NumericError::OutOfRange)?;

        if scale <= 0 {
            let zeros = scale.unsigned_abs();
            if zeros > 38 {
                return Err(NumericError::OutOfRange);
            }
            for _ in 0..zeros {
                magnitude = magnitude
                    .checked_mul(10)
                    .ok_or(NumericError::ConversionOverflow)?;
            }
            return Self::from_magnitudes(negative, magnitude, 1, NumericError::OutOfRange);
        }

        let mut twos = u64::try_from(scale).map_err(|_| NumericError::OutOfRange)?;
        let mut fives = twos;
        while twos > 0 && magnitude % 2 == 0 {
            magnitude /= 2;
            twos -= 1;
        }
        while fives > 0 && magnitude % 5 == 0 {
            magnitude /= 5;
            fives -= 1;
        }
        let two_factor = checked_pow_u128(2, twos)?;
        let five_factor = checked_pow_u128(5, fives)?;
        let denominator = two_factor
            .checked_mul(five_factor)
            .ok_or(NumericError::OutOfRange)?;
        Self::from_magnitudes(negative, magnitude, denominator, NumericError::OutOfRange)
    }

    /// Convert integer font units to em: `units / units_per_em`.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ZeroDenominator`] when `units_per_em == 0`.
    pub fn from_font_units(units: i64, units_per_em: u16) -> Result<Self, NumericError> {
        Self::from_parts(i128::from(units), i128::from(units_per_em))
    }

    pub(crate) fn from_font_units_nonzero(units: i64, units_per_em: NonZeroU16) -> Self {
        let num = i128::from(units);
        if num == 0 {
            return Self::zero();
        }
        let den = i128::from(units_per_em.get());
        let g = gcd_u(num.unsigned_abs(), den as u128) as i128;
        Self {
            num: num / g,
            den: den / g,
        }
    }

    /// One math unit (mu). TeX: `18 mu = 1 em`.
    #[must_use]
    pub const fn mu() -> Self {
        Self { num: 1, den: 18 }
    }

    /// Convert this em value to mu (`* 18`).
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the exact result is
    /// outside the supported [`Dim`] range.
    pub fn to_mu(&self) -> Result<Self, NumericError> {
        self.checked_mul(&Self::from_i64(18))
    }

    /// Convert a mu value to em (`/ 18`).
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the exact result is
    /// outside the supported [`Dim`] range.
    pub fn from_mu(mu: &Self) -> Result<Self, NumericError> {
        mu.checked_div(&Self::from_i64(18))
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den,
        }
    }

    /// Maximum of two dimensions.
    #[must_use]
    pub fn max_ref(&self, other: &Self) -> Self {
        if self < other {
            other.clone()
        } else {
            self.clone()
        }
    }

    /// Minimum of two dimensions.
    #[must_use]
    pub fn min_ref(&self, other: &Self) -> Self {
        if self > other {
            other.clone()
        } else {
            self.clone()
        }
    }

    /// `max(self, 0)`.
    #[must_use]
    pub fn clamp_nonneg(&self) -> Self {
        self.max_ref(&Self::zero())
    }

    /// True when the value equals zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.num == 0
    }

    /// Exact numeric equality. Kept as a compatibility helper for callers that
    /// used the pre-B1 comparison API. Canonical representation makes this
    /// equivalent to `PartialEq`.
    #[must_use]
    pub fn eq_dim(&self, other: &Self) -> bool {
        self == other
    }

    /// The exact rational value in em, as `(numerator, denominator)`.
    ///
    /// The denominator is positive and the fraction is in lowest terms.
    #[must_use]
    pub const fn as_ratio(&self) -> (i128, i128) {
        (self.num, self.den)
    }

    /// Deterministic decimal expansion with at most 24 fractional digits.
    /// Longer terminating expansions and non-terminating expansions are
    /// truncated after the 24th fractional digit for the existing gold format.
    #[must_use]
    pub fn to_dec_string(&self) -> String {
        format_rational(self.num, self.den)
    }

    /// Checked exact addition.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the canonical exact
    /// result is outside the supported [`Dim`] range.
    pub fn checked_add(&self, other: &Self) -> Result<Self, NumericError> {
        self.checked_add_sub(other, false)
    }

    /// Checked exact subtraction.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the canonical exact
    /// result is outside the supported [`Dim`] range.
    pub fn checked_sub(&self, other: &Self) -> Result<Self, NumericError> {
        self.checked_add_sub(other, true)
    }

    fn checked_add_sub(&self, other: &Self, subtract: bool) -> Result<Self, NumericError> {
        let g = gcd_u(self.den as u128, other.den as u128);
        let g_i = g as i128;
        let lhs_factor = other.den / g_i;
        let rhs_factor = self.den / g_i;

        let fast = self
            .num
            .checked_mul(lhs_factor)
            .zip(other.num.checked_mul(rhs_factor))
            .and_then(|(lhs, rhs)| {
                if subtract {
                    lhs.checked_sub(rhs)
                } else {
                    lhs.checked_add(rhs)
                }
            });
        if let Some(numerator) = fast {
            return Self::finish_add_sub(numerator, rhs_factor, other.den, g);
        }

        self.checked_add_sub_wide(other, subtract, lhs_factor, rhs_factor, g)
    }

    fn finish_add_sub(
        numerator: i128,
        rhs_factor: i128,
        other_den: i128,
        g: u128,
    ) -> Result<Self, NumericError> {
        if numerator == 0 {
            return Ok(Self::zero());
        }
        let reduce = gcd_u(numerator.unsigned_abs(), g);
        let reduce_i = reduce as i128;
        let numerator = numerator / reduce_i;
        let denominator = rhs_factor
            .checked_mul(other_den / reduce_i)
            .ok_or(NumericError::ArithmeticOverflow)?;
        Self::from_arithmetic_parts(numerator, denominator)
    }

    fn checked_add_sub_wide(
        &self,
        other: &Self,
        subtract: bool,
        lhs_factor: i128,
        rhs_factor: i128,
        g: u128,
    ) -> Result<Self, NumericError> {
        let lhs = wide_mul_u128(self.num.unsigned_abs(), lhs_factor as u128)
            .ok_or(NumericError::ArithmeticOverflow)?;
        let rhs = wide_mul_u128(other.num.unsigned_abs(), rhs_factor as u128)
            .ok_or(NumericError::ArithmeticOverflow)?;
        let lhs_negative = self.num < 0;
        let rhs_negative = (other.num < 0) ^ subtract;

        let (negative, numerator) = if lhs_negative == rhs_negative {
            (
                lhs_negative,
                lhs.checked_add(rhs)
                    .ok_or(NumericError::ArithmeticOverflow)?,
            )
        } else {
            match lhs.cmp(&rhs) {
                Ordering::Less => (
                    rhs_negative,
                    rhs.checked_sub(lhs)
                        .ok_or(NumericError::ArithmeticOverflow)?,
                ),
                Ordering::Equal => return Ok(Self::zero()),
                Ordering::Greater => (
                    lhs_negative,
                    lhs.checked_sub(rhs)
                        .ok_or(NumericError::ArithmeticOverflow)?,
                ),
            }
        };

        let g_nonzero = NonZeroU128::new(g).ok_or(NumericError::ArithmeticOverflow)?;
        let reduce = gcd_u(numerator.rem_u128(g_nonzero), g);
        let reduce_nonzero = NonZeroU128::new(reduce).ok_or(NumericError::ArithmeticOverflow)?;
        let magnitude = numerator
            .div_exact_to_u128(reduce_nonzero)
            .filter(|value| *value <= I128_MAX_U)
            .ok_or(NumericError::ArithmeticOverflow)?;
        let reduce_i = reduce as i128;
        let denominator = rhs_factor
            .checked_mul(other.den / reduce_i)
            .ok_or(NumericError::ArithmeticOverflow)?;
        let signed = if negative {
            -(magnitude as i128)
        } else {
            magnitude as i128
        };
        Self::from_arithmetic_parts(signed, denominator)
    }

    /// Checked exact multiplication with cross-cancellation before products.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ArithmeticOverflow`] if the canonical exact
    /// result is outside the supported [`Dim`] range.
    pub fn checked_mul(&self, other: &Self) -> Result<Self, NumericError> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero());
        }
        let left_cancel = gcd_u(self.num.unsigned_abs(), other.den as u128);
        let right_cancel = gcd_u(other.num.unsigned_abs(), self.den as u128);
        let left_cancel = left_cancel as i128;
        let right_cancel = right_cancel as i128;
        let lhs_num = self.num / left_cancel;
        let rhs_num = other.num / right_cancel;
        let lhs_den = self.den / right_cancel;
        let rhs_den = other.den / left_cancel;
        let numerator = lhs_num
            .checked_mul(rhs_num)
            .ok_or(NumericError::ArithmeticOverflow)?;
        let denominator = lhs_den
            .checked_mul(rhs_den)
            .ok_or(NumericError::ArithmeticOverflow)?;
        Self::from_arithmetic_parts(numerator, denominator)
    }

    /// Checked exact division with cross-cancellation before products.
    ///
    /// # Errors
    ///
    /// Returns [`NumericError::ZeroDenominator`] when `other` is zero and
    /// [`NumericError::ArithmeticOverflow`] if the canonical exact result is
    /// outside the supported [`Dim`] range.
    pub fn checked_div(&self, other: &Self) -> Result<Self, NumericError> {
        if other.is_zero() {
            return Err(NumericError::ZeroDenominator);
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        let numerator_cancel = gcd_u(self.num.unsigned_abs(), other.num.unsigned_abs());
        let denominator_cancel = gcd_u(self.den as u128, other.den as u128);
        let numerator_cancel = numerator_cancel as i128;
        let denominator_cancel = denominator_cancel as i128;
        let lhs_num = self.num / numerator_cancel;
        let rhs_num = other.num / numerator_cancel;
        let lhs_den = self.den / denominator_cancel;
        let rhs_den = other.den / denominator_cancel;
        let numerator = lhs_num
            .checked_mul(rhs_den)
            .ok_or(NumericError::ArithmeticOverflow)?;
        let denominator = lhs_den
            .checked_mul(rhs_num)
            .ok_or(NumericError::ArithmeticOverflow)?;
        Self::from_arithmetic_parts(numerator, denominator)
    }
}

fn validate_decimal_exponent(raw: &str) -> Result<(), NumericError> {
    if raw.is_empty() {
        return Err(NumericError::InvalidDecimal);
    }
    let digits = match raw.as_bytes()[0] {
        b'-' | b'+' => &raw[1..],
        _ => raw,
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(NumericError::InvalidDecimal);
    }
    Ok(())
}

fn parse_decimal_exponent(raw: &str) -> Result<i64, NumericError> {
    validate_decimal_exponent(raw)?;
    let (negative, digits) = match raw.as_bytes()[0] {
        b'-' => (true, &raw[1..]),
        b'+' => (false, &raw[1..]),
        _ => (false, raw),
    };
    let mut value = 0i64;
    for b in digits.bytes() {
        value = value
            .checked_mul(10)
            .and_then(|n| n.checked_add(i64::from(b - b'0')))
            .ok_or(NumericError::OutOfRange)?;
    }
    if negative {
        value.checked_neg().ok_or(NumericError::OutOfRange)
    } else {
        Ok(value)
    }
}

fn checked_pow_u128(base: u128, exponent: u64) -> Result<u128, NumericError> {
    let mut result = 1u128;
    let mut factor = base;
    let mut remaining = exponent;
    while remaining != 0 {
        if remaining & 1 != 0 {
            result = result.checked_mul(factor).ok_or(NumericError::OutOfRange)?;
        }
        remaining >>= 1;
        if remaining != 0 {
            factor = factor.checked_mul(factor).ok_or(NumericError::OutOfRange)?;
        }
    }
    Ok(result)
}

fn format_rational(num: i128, den: i128) -> String {
    if num == 0 {
        return "0.0".into();
    }
    let neg = num < 0;
    let mut n = num.unsigned_abs();
    let d = den as u128;
    let ip = n / d;
    n %= d;
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    if n == 0 {
        s.push_str(&ip.to_string());
        s.push_str(".0");
        return s;
    }
    s.push_str(&ip.to_string());
    s.push('.');
    // Preserve long division without forming n * 10, which can overflow.
    for _ in 0..24 {
        if n == 0 {
            break;
        }
        let mut digit = 0u8;
        let mut next = 0u128;
        for _ in 0..10 {
            next += n;
            if next >= d {
                next -= d;
                digit += 1;
            }
        }
        s.push(char::from(b'0' + digit));
        n = next;
    }
    // Trim trailing zeros but keep one fractional digit.
    while s.ends_with('0') && !s.ends_with(".0") {
        s.pop();
    }
    s
}

impl Ord for Dim {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.num.cmp(&0), other.num.cmp(&0)) {
            (Ordering::Less, Ordering::Less) => reverse(cmp_unsigned_rational(
                self.num.unsigned_abs(),
                self.den as u128,
                other.num.unsigned_abs(),
                other.den as u128,
            )),
            (Ordering::Less, _) => Ordering::Less,
            (_, Ordering::Less) => Ordering::Greater,
            (Ordering::Equal, Ordering::Equal) => Ordering::Equal,
            (Ordering::Equal, Ordering::Greater) => Ordering::Less,
            (Ordering::Greater, Ordering::Equal) => Ordering::Greater,
            (Ordering::Greater, Ordering::Greater) => cmp_unsigned_rational(
                self.num as u128,
                self.den as u128,
                other.num as u128,
                other.den as u128,
            ),
        }
    }
}

impl PartialOrd for Dim {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_dec_string())
    }
}

impl Neg for Dim {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            num: -self.num,
            den: self.den,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cmp::Ordering;
    use core::num::NonZeroU128;

    use super::{wide_mul_u128, Dim, I128_MAX_U};
    use crate::error::NumericError;

    #[derive(Clone, Copy)]
    struct RefRat {
        num: i128,
        den: i128,
    }

    impl RefRat {
        fn new(num: i64, den: i64) -> Self {
            assert_ne!(den, 0);
            let mut num = i128::from(num);
            let mut den = i128::from(den);
            if den < 0 {
                num = -num;
                den = -den;
            }
            Self { num, den }
        }

        fn add(self, other: Self) -> Self {
            Self {
                num: self.num * other.den + other.num * self.den,
                den: self.den * other.den,
            }
        }

        fn sub(self, other: Self) -> Self {
            Self {
                num: self.num * other.den - other.num * self.den,
                den: self.den * other.den,
            }
        }

        fn mul(self, other: Self) -> Self {
            Self {
                num: self.num * other.num,
                den: self.den * other.den,
            }
        }

        fn div(self, other: Self) -> Self {
            assert_ne!(other.num, 0);
            let mut num = self.num * other.den;
            let mut den = self.den * other.num;
            if den < 0 {
                num = -num;
                den = -den;
            }
            Self { num, den }
        }

        fn cmp(self, other: Self) -> Ordering {
            (self.num * other.den).cmp(&(other.num * self.den))
        }

        fn equals_dim(self, dim: &Dim) -> bool {
            let (num, den) = dim.as_ratio();
            self.num * den == num * self.den
        }
    }

    #[test]
    fn half_and_font_units() {
        let half = Dim::ratio(1, 2).unwrap();
        assert_eq!(half, Dim::one().checked_div(&Dim::from_i64(2)).unwrap());
        let width = Dim::from_font_units(479, 1000).unwrap();
        assert_eq!(width.to_dec_string(), "0.479");
        assert_eq!(
            Dim::from_i64(12)
                .checked_mul(&width)
                .unwrap()
                .to_dec_string(),
            "5.748"
        );
    }

    #[test]
    fn canonical_construction_and_zero() {
        assert_eq!(Dim::ratio(2, 6).unwrap().as_ratio(), (1, 3));
        assert_eq!(Dim::ratio(2, 6).unwrap(), Dim::ratio(1, 3).unwrap());
        assert_eq!(Dim::ratio(1, -2).unwrap().as_ratio(), (-1, 2));
        assert_eq!(Dim::ratio(-1, -2).unwrap().as_ratio(), (1, 2));
        assert_eq!(Dim::ratio(0, -99).unwrap().as_ratio(), (0, 1));
        assert_eq!(Dim::ratio(1, 0), Err(NumericError::ZeroDenominator));
    }

    #[test]
    fn canonical_ratio_round_trip_preserves_invariant() {
        for value in [
            Dim::zero(),
            Dim::from_parts(i128::MAX, i128::MAX - 1).unwrap(),
            Dim::from_parts(-42, -56).unwrap(),
            Dim::from_parts(-42, 56).unwrap(),
        ] {
            let (num, den) = value.as_ratio();
            assert!(den > 0);
            assert_eq!(Dim::from_parts(num, den).unwrap(), value);
        }
    }

    #[test]
    fn i128_boundary_adjacent_values_and_total_negation() {
        let max = Dim::from_parts(i128::MAX, 1).unwrap();
        let near_min = Dim::from_parts(-i128::MAX, 1).unwrap();
        assert_eq!((-max.clone()).as_ratio(), near_min.as_ratio());
        assert_eq!((-near_min).as_ratio(), max.as_ratio());
        assert_eq!(Dim::from_parts(i128::MIN, 1), Err(NumericError::OutOfRange));
        assert_eq!(
            Dim::from_parts(i128::MIN, 2).unwrap().as_ratio(),
            (-(1i128 << 126), 1)
        );
        assert_eq!(
            Dim::from_parts(2, i128::MIN).unwrap().as_ratio(),
            (-1, 1i128 << 126)
        );
        assert_eq!(
            Dim::from_parts(i128::MIN, i128::MIN).unwrap().as_ratio(),
            (1, 1)
        );
    }

    #[test]
    fn large_reducible_fractions_do_not_overflow() {
        let x = Dim::from_parts(i128::MAX - 1, i128::MAX - 1).unwrap();
        assert_eq!(x, Dim::one());

        let factor = 1i128 << 100;
        let a = Dim::from_parts(3 * factor, 5 * factor).unwrap();
        let b = Dim::from_parts(5 * factor, 7 * factor).unwrap();
        let product = a.checked_mul(&b).unwrap();
        assert_eq!(product.as_ratio(), (3, 7));

        let cross_left = Dim::from_parts(i128::MAX, 3).unwrap();
        let cross_right = Dim::from_parts(3, i128::MAX).unwrap();
        assert_eq!(cross_left.checked_mul(&cross_right).unwrap(), Dim::one());
        assert_eq!(cross_left.checked_div(&cross_left).unwrap(), Dim::one());
    }

    #[test]
    fn denominator_gcd_reduction_avoids_false_overflow() {
        let den = i128::MAX - 1;
        let a = Dim::from_parts(1, den).unwrap();
        let b = Dim::from_parts(1, den).unwrap();
        assert_eq!(a.checked_add(&b).unwrap().as_ratio(), (1, den / 2));
        assert_eq!(a.checked_sub(&b).unwrap(), Dim::zero());
    }

    #[test]
    fn post_add_reduction_avoids_false_overflow() {
        let half = Dim::from_parts(1, 2).unwrap();
        let max_halves = Dim::from_parts(i128::MAX, 2).unwrap();
        let expected = Dim::from_parts(1i128 << 126, 1).unwrap();
        assert_eq!(half.checked_add(&max_halves).unwrap(), expected);

        let negative_max_halves = -max_halves;
        assert_eq!(half.checked_sub(&negative_max_halves).unwrap(), expected);
    }

    #[test]
    fn wide_fallback_primitives_preserve_full_product() {
        let product = wide_mul_u128(I128_MAX_U, I128_MAX_U).unwrap();
        assert_eq!(product.hi, (1u128 << 126) - 1);
        assert_eq!(product.lo, 1);

        let doubled = wide_mul_u128(I128_MAX_U, 2).unwrap();
        assert_eq!(
            doubled.div_exact_to_u128(NonZeroU128::new(2).unwrap()),
            Some(I128_MAX_U)
        );
        assert_eq!(
            doubled.rem_u128(NonZeroU128::new(97).unwrap()),
            (I128_MAX_U * 2) % 97
        );
    }

    #[test]
    fn checked_arithmetic_reports_overflow_and_zero_divisor() {
        let max = Dim::from_parts(i128::MAX, 1).unwrap();
        assert_eq!(
            max.checked_add(&Dim::one()),
            Err(NumericError::ArithmeticOverflow)
        );
        assert_eq!(
            max.checked_mul(&Dim::from_i64(2)),
            Err(NumericError::ArithmeticOverflow)
        );
        assert_eq!(
            Dim::one().checked_div(&Dim::zero()),
            Err(NumericError::ZeroDenominator)
        );
    }

    #[test]
    fn decimal_parsing_and_exponents() {
        assert_eq!(Dim::parse(".5").unwrap().as_ratio(), (1, 2));
        assert_eq!(Dim::parse("-12.50").unwrap().as_ratio(), (-25, 2));
        assert_eq!(Dim::parse("1.25e2").unwrap().as_ratio(), (125, 1));
        assert_eq!(Dim::parse("125e-2").unwrap().as_ratio(), (5, 4));
        assert_eq!(Dim::parse("0e999999999999999999999").unwrap(), Dim::zero());
        let redundant_fraction_zeros = format!("0.1{}", "0".repeat(100));
        assert_eq!(
            Dim::parse(&redundant_fraction_zeros).unwrap().as_ratio(),
            (1, 10)
        );
        let exponent_cancelled_zeros = format!("1{}e-100", "0".repeat(100));
        assert_eq!(Dim::parse(&exponent_cancelled_zeros).unwrap(), Dim::one());
        assert_eq!(Dim::parse("nan"), Err(NumericError::InvalidDecimal));
        assert_eq!(Dim::parse("1.2.3"), Err(NumericError::InvalidDecimal));
        assert_eq!(Dim::parse("1e"), Err(NumericError::InvalidDecimal));
        assert_eq!(
            Dim::parse("340282366920938463463374607431768211456"),
            Err(NumericError::ConversionOverflow)
        );
        assert_eq!(
            Dim::parse("1e999999999999999999999"),
            Err(NumericError::OutOfRange)
        );
    }

    #[test]
    fn font_units_reject_zero_denominator() {
        assert_eq!(
            Dim::from_font_units(1, 0),
            Err(NumericError::ZeroDenominator)
        );
    }

    #[test]
    fn terminating_decimal_round_trip_is_canonical() {
        for value in [
            Dim::zero(),
            Dim::ratio(1, 2).unwrap(),
            Dim::ratio(-25, 8).unwrap(),
            Dim::ratio(125, 16).unwrap(),
        ] {
            assert_eq!(Dim::parse(&value.to_dec_string()).unwrap(), value);
        }
    }

    #[test]
    fn comparison_is_total_at_large_values() {
        let a = Dim::from_parts(i128::MAX, i128::MAX - 1).unwrap();
        let b = Dim::from_parts(i128::MAX - 1, i128::MAX - 2).unwrap();
        assert_eq!(a.cmp(&b), Ordering::Less);
        assert_eq!((-a).cmp(&(-b)), Ordering::Greater);
    }

    #[test]
    fn decimal_format_avoids_large_remainder_overflow() {
        let near_one = Dim::ratio(i64::MAX - 1, i64::MAX).unwrap();
        let squared = near_one.checked_mul(&near_one).unwrap();
        assert_eq!(squared.to_dec_string(), "0.999999999999999999783159");
    }

    #[test]
    fn arithmetic_matches_independent_property_reference() {
        // Deterministic property sweep. Keep operands below 2^31 so the slow
        // reference cross-products remain independently safe in i128.
        let mut state = 0x6a09_e667_f3bc_c909u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state
        };

        for _ in 0..20_000 {
            let signed = |bits: u64| -> i64 {
                let magnitude = i64::try_from((bits >> 1) & 0x7fff_ffff).unwrap();
                if bits & 1 == 0 {
                    magnitude
                } else {
                    -magnitude
                }
            };
            let positive = |bits: u64| -> i64 { i64::try_from((bits & 0x7fff_ffff) + 1).unwrap() };

            let a_num = signed(next());
            let a_den = positive(next());
            let b_num = signed(next());
            let b_den = positive(next());
            let a = Dim::ratio(a_num, a_den).unwrap();
            let b = Dim::ratio(b_num, b_den).unwrap();
            let ra = RefRat::new(a_num, a_den);
            let rb = RefRat::new(b_num, b_den);

            assert!(ra.add(rb).equals_dim(&a.checked_add(&b).unwrap()));
            assert!(ra.sub(rb).equals_dim(&a.checked_sub(&b).unwrap()));
            assert!(ra.mul(rb).equals_dim(&a.checked_mul(&b).unwrap()));
            assert_eq!(a.cmp(&b), ra.cmp(rb));
            if b_num != 0 {
                assert!(ra.div(rb).equals_dim(&a.checked_div(&b).unwrap()));
            }
        }
    }

    #[test]
    fn arithmetic_matches_independent_small_reference() {
        for a_num in -12i64..=12 {
            for a_den in 1i64..=12 {
                let a = Dim::ratio(a_num, a_den).unwrap();
                let ra = RefRat::new(a_num, a_den);
                for b_num in -12i64..=12 {
                    for b_den in 1i64..=12 {
                        let b = Dim::ratio(b_num, b_den).unwrap();
                        let rb = RefRat::new(b_num, b_den);
                        assert!(ra.add(rb).equals_dim(&a.checked_add(&b).unwrap()));
                        assert!(ra.sub(rb).equals_dim(&a.checked_sub(&b).unwrap()));
                        assert!(ra.mul(rb).equals_dim(&a.checked_mul(&b).unwrap()));
                        assert_eq!(a.cmp(&b), ra.cmp(rb));
                        if b_num != 0 {
                            assert!(ra.div(rb).equals_dim(&a.checked_div(&b).unwrap()));
                        }
                    }
                }
            }
        }
    }
}
