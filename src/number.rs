use core::{cmp::Ordering, convert::TryInto, ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign}};

use alloc::{vec, vec::Vec, string::ToString};
use num_integer::{Roots, Integer};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero, Signed};
use rust_decimal::{Decimal, MathematicalOps};

use crate::{decimal_ext::DecimalExtensions, node::unstructured::Serializable, error::MathsError};

/// Represents the accuracy of a `Decimal` number, based on how it was created.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum DecimalAccuracy {
    /// This number was derived from user input, and has only been used through exact operations.
    Exact,

    /// This number has been passed through operations which compute their results approximately.
    Approximation,
}

impl DecimalAccuracy {
    /// Given another accuracy, returns the least accurate of the two.
    pub fn combine(self, other: Self) -> Self {
        match (self, other) {
            (DecimalAccuracy::Exact, DecimalAccuracy::Exact) => DecimalAccuracy::Exact,
            _ => DecimalAccuracy::Approximation,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Number {
    Decimal(Decimal, DecimalAccuracy),
    Rational(i64, i64),
}

impl Number {
    /// Gets the accuracy of this number, if it is a `Decimal`.
    /// 
    /// `Rational` numbers always return `DecimalAccuracy::Exact`.
    pub fn accuracy(&self) -> DecimalAccuracy {
        match self {
            Number::Decimal(_, a) => *a,
            Number::Rational(_, _) => DecimalAccuracy::Exact,
        }
    }

    /// Converts this number to a decimal:
    ///   - For `Decimal`, this simply unwraps the variant.
    ///   - For `Rational`, this divides the numerator by the denominator after converting both to
    ///     decimals.
    pub fn to_decimal(&self) -> Decimal {
        match self {
            Number::Decimal(d, _) => *d,
            Number::Rational(numer, denom)
                => Decimal::from_i64(*numer).unwrap() / Decimal::from_i64(*denom).unwrap(),
        }
    }

    /// Utility function which gets the greatest common denominator of two numbers. 
    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 {
            return a;
        }

        Self::gcd(b, a % b)
    }

    /// Utility function which gets the lowest common multiple of two numbers.
    fn lcm(a: i64, b: i64) -> i64 {
        (a * b).abs() / Self::gcd(a, b)
    }

    /// Given two `Rational` numbers, returns the same two numbers in the form (self, other), except
    /// that both numbers have the same denominator.
    ///
    /// Panics if either of the numbers is not rational.
    fn to_common_with(self, other: Number) -> (Number, Number) {
        if let (Self::Rational(ln, ld), Self::Rational(rn, rd)) = (self, other) {
            let new_denominator = Self::lcm(ld, rd);
            let ln = (new_denominator / ld) * ln;
            let rn = (new_denominator / rd) * rn;

            (
                Self::Rational(ln, new_denominator),
                Self::Rational(rn, new_denominator),
            )
        } else {
            panic!("both numbers must be rational");
        }
    }

    /// Assumes that this is a `Rational` number and returns its numerator, otherwise panics.
    fn numerator(&self) -> i64 {
        if let Self::Rational(n, _) = self {
            *n
        } else {
            panic!("not rational")
        }
    }

    /// Assumes that this is a `Rational` number and returns its denominator, otherwise panics.
    fn denominator(&self) -> i64 {
        if let Self::Rational(_, d) = self {
            *d
        } else {
            panic!("not rational")
        }
    }

    /// Simplifies this number:
    ///   - For `Decimal`, this normalises the number, and then performs inaccuracy correction.
    ///     This is a potentially lossy operation, but more often that not results in better output.
    ///   - For `Rational`, this divides the numerator and denominator by their GCD. Also ensures
    ///     that any negative sign is on the numerator, not the denominator.
    pub fn simplify(&self) -> Number {
        match self {
            Self::Decimal(d, a) => Self::Decimal(d.normalize(), *a).correct_inaccuracy(),

            Self::Rational(numer, denom) => {
                let sign = match (*numer < 0, *denom < 0) {
                    (false, false) => 1, // Neither negative
                    (true, false) | (false, true) => -1, // One negative
                    (true, true) => 1, // Both negative, cancels out
                };

                let (numer, denom) = (numer.abs(), denom.abs());

                let gcd = Self::gcd(numer, denom);
                Self::Rational(sign * (numer / gcd), denom / gcd)
            }
        }
    }

    /// Returns the reciprocal of this number.
    pub fn reciprocal(&self) -> Number {
        match self {
            Self::Decimal(d, a) => Self::Decimal(Decimal::one() / d, *a),
            Self::Rational(numer, denom) => Self::Rational(*denom, *numer),
        }
    }

    /// If this is a whole number, returns it. Otherwise returns None.
    pub fn to_whole(&self) -> Option<i64> {
        match self {
            Self::Decimal(d, _)
                => if d.is_whole() { d.floor().to_i64() } else { None },
            Self::Rational(numer, denom)
                => if numer % denom == 0 { Some(numer / denom) } else { None },
        }
    }

    /// Raises this number to an integer power.
    pub fn powi(&self, exp: i64) -> Number {
        let mut n = *self;

        // Repeatedly multiply 
        for _ in 1..exp.abs() {
            n = n * *self;
        }
        
        // Reciprocal for negative powers
        if exp < 0 {
            n.reciprocal()
        } else {
            n
        }
    }

    /// Adds this number to another number, or returns an error if an overflow occurs.
    pub fn checked_add(&self, other: Number) -> Result<Number, MathsError> {
        if let (l@Self::Rational(_, _), r@Self::Rational(_, _)) = (self, other) {
            let (l, r) = l.to_common_with(r);

            Ok(Number::Rational(
                l.numerator().checked_add(r.numerator()).ok_or(MathsError::Overflow)?,
                l.denominator(),
            ).simplify())
        } else {
            Ok(Number::Decimal(
                self.to_decimal().checked_add(other.to_decimal()).ok_or(MathsError::Overflow)?,
                self.accuracy().combine(other.accuracy()),
            ))
        }
    }

    /// Subtracts one number from another number, or returns an error if an overflow occurs.
    pub fn checked_sub(&self, other: Number) -> Result<Number, MathsError> {
        self.checked_add(-other)
    }

    /// Multiplies this number with another number, or returns an error if an overflow occurs.
    pub fn checked_mul(&self, other: Number) -> Result<Number, MathsError> {
        if let (Self::Rational(ln, ld), Self::Rational(rn, rd)) = (self, other) {
            Ok(Number::Rational(
                ln.checked_mul(rn).ok_or(MathsError::Overflow)?,
                ld.checked_mul(rd).ok_or(MathsError::Overflow)?,
            ).simplify())
        } else {
            Ok(Number::Decimal(
                self.to_decimal().checked_mul(other.to_decimal()).ok_or(MathsError::Overflow)?,
                self.accuracy().combine(other.accuracy()),
            ).simplify())
        }
    }

    /// Divides this number by another number, or returns an error if the divisor is zero.
    pub fn checked_div(&self, other: Number) -> Result<Number, MathsError> {
        if other.is_zero() {
            Err(MathsError::DivisionByZero)
        } else {
            Ok(*self / other)
        }
    }

    /// Raises this number to the power of another number.
    pub fn checked_pow(&self, power: Number) -> Result<Number, MathsError> {
        // If both power and base are rational, we can get a bit more accuracy by breaking it down
        if let (Self::Rational(bn, bd), Self::Rational(pn, pd)) = (self, power) {
            // Can only keep as rational if (power denominator)th root of both base numerator and
            // denominator are integers
            if (bn.is_negative() && pd.is_even()) || (bd.is_negative() && pd.is_even()) {
                return Err(MathsError::Imaginary)
            }

            // TODO: handle panics in `nth_root`
            let bn_pd_nth_root = bn.nth_root(pd.try_into().map_err(|_| MathsError::Overflow)?);
            let bd_pd_nth_root = bd.nth_root(pd.try_into().map_err(|_| MathsError::Overflow)?);
            if bn_pd_nth_root.pow(pd.abs().try_into().map_err(|_| MathsError::Overflow)?) == *bn
               && bd_pd_nth_root.pow(pd.abs().try_into().map_err(|_| MathsError::Overflow)?) == *bd {

                let mut result = Number::Rational(
                    bn_pd_nth_root.pow(pn.abs().try_into().map_err(|_| MathsError::Overflow)?), 
                    bd_pd_nth_root.pow(pn.abs().try_into().map_err(|_| MathsError::Overflow)?), 
                );

                if pn < 0 {
                    result = result.reciprocal();
                }

                return Ok(result)
            }
        }

        Ok(Number::Decimal(
            self.to_decimal().checked_powd(power.to_decimal()).ok_or(MathsError::Overflow)?,
            DecimalAccuracy::Approximation,
        ))
    }

    /// The minimum number of repeated digits where `correct_float` will trigger a truncation.
    /// (This number wasn't picked for any particular reason, more just what felt about right!)
    const CORRECT_FLOAT_DIGIT_THRESHOLD: usize = 10;

    /// Attempts to correct inaccuracies in this number introduced by imprecise operations.
    /// 
    /// For example:
    ///   - 1.14000000000000003 would be corrected to 1.14 (`Decimal`)
    ///   - 1.9999999999997 would be corrected to 2 (`Rational`)
    /// 
    /// This only has an effect for `Decimal` numbers with `DecimalAccuracy::Approximation` - others
    /// are returned unchanged.
    /// 
    /// If the intended number does actually look like one of these imprecise results, then this
    /// could result in a *loss* of precision instead.
    pub fn correct_inaccuracy(&self) -> Number {
        match self {
            Number::Decimal(d, DecimalAccuracy::Approximation) if !d.is_whole() => {
                // Iterate over digits of the fractional part, as a string
                // This is pretty expensive, but it's a lot easier implementation-wise than dealing
                // with leading zeroes when splitting off the fractional part into an integer
                let d_str = d.to_string();
                let fractional_digits = d_str
                    .chars()
                    .skip_while(|c| *c != '.')
                    .skip(1)
                    .map(|d| d.to_digit(10).unwrap())
                    .collect::<Vec<_>>();

                // Look for repetitions of "extreme" digits (0 or 9)
                #[derive(Debug, Copy, Clone)] struct Repeat { start: usize, digit: u32, length: usize }
                let mut current_repeat: Option<Repeat> = None;
                for (i, digit) in fractional_digits.iter().enumerate() {
                    match current_repeat {
                        // If this digit matches the current repeat, increment its length and skip
                        // the rest of the iteration, or break if we reached the threshold
                        Some(ref mut repeat) if repeat.digit == *digit => {
                            repeat.length += 1;
                            if repeat.length >= Self::CORRECT_FLOAT_DIGIT_THRESHOLD { break }
                            continue
                        }

                        // If the digit doesn't match the current repeat, cancel the repeat
                        // (We don't skip the rest of the iteration because we might have gone from
                        // one extreme digit to the other, so need to check to start a new repeat)
                        Some(_) => {
                            current_repeat = None;
                        }

                        // If there's no repeat ongoing, keep going
                        None => (),
                    }

                    // Start new repeat if we have an extreme digit
                    if *digit == 0 || *digit == 9 {
                        current_repeat = Some(Repeat { start: i, digit: *digit, length: 1 });
                    }
                }

                // If the final repeat exceeds the repeat threshold, let's truncate our number!
                if let Some(repeat) = current_repeat
                    && repeat.length >= Self::CORRECT_FLOAT_DIGIT_THRESHOLD
                {
                    // If the repetition began right at the start, we need to operate on the whole
                    // part
                    if repeat.start == 0 {
                        return Number::Decimal(match repeat.digit {
                            0 => d.trunc(), // 1.000... -> 1
                            9 => d.trunc() + d.signum(), // 1.999... -> 2

                            _ => unreachable!(),
                        }, DecimalAccuracy::Approximation)
                    }

                    // Otherwise, there's still a fractional part, and we're operating on that
                    // Let's construct a new mantissa and exponent, starting with just the whole
                    // part
                    let mut new_mantissa = d.trunc().to_i64().unwrap().abs();
                    let mut new_scale = 0;

                    // Insert the digits before the repetition started
                    for digit in fractional_digits.iter().take(repeat.start) {
                        new_mantissa *= 10;
                        new_mantissa += *digit as i64;
                        new_scale += 1;
                    }

                    // Re-apply sign
                    let sign = d.signum().to_i64().unwrap();
                    new_mantissa *= sign;

                    // Construct new decimal and act on repeated digit
                    return Number::Decimal(match repeat.digit {
                        0 => Decimal::new(new_mantissa, new_scale), // 1.2000... -> 1.2
                        9 => Decimal::new(new_mantissa + sign, new_scale), // 1.2999 -> 1.3
                        _ => unreachable!(),
                    }, DecimalAccuracy::Approximation)
                }

                // No correction to do
                self.clone()
            },
            _ => self.clone(),
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_decimal().cmp(&other.to_decimal())
    }
}

impl From<Decimal> for Number {
    fn from(d: Decimal) -> Self {
        Self::Decimal(d, DecimalAccuracy::Exact)
    }
}

impl From<i64> for Number {
    fn from(i: i64) -> Self {
        Self::Rational(i, 1)
    }
}

impl Neg for Number {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::Rational(n, d) => Number::Rational(-n, d).simplify(),
            Self::Decimal(d, a) => Self::Decimal(-d, a),
        }
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl AddAssign for Number {
    fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + -rhs
    }
}

impl SubAssign for Number {
    fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
}

impl Mul for Number {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap()
    }
}

impl MulAssign for Number {
    fn mul_assign(&mut self, rhs: Self) { *self = *self * rhs; }
}

impl Div for Number {
    type Output = Self;
    
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.reciprocal()
    }
}

impl DivAssign for Number {
    fn div_assign(&mut self, rhs: Self) { *self = *self / rhs; }
}

impl Zero for Number {
    fn zero() -> Self {
        Self::Rational(0, 1)
    }

    fn is_zero(&self) -> bool {
        match *self {
            Self::Decimal(d, _) => d.is_zero(),
            Self::Rational(n, _) => n.is_zero(),
        }
    }
}

impl One for Number {
    fn one() -> Self {
        Self::Rational(1, 1)
    }

    fn is_one(&self) -> bool {
        match *self {
            Self::Decimal(d, _) => d.is_one(),
            Self::Rational(n, d) => n == d,
        }
    }
}

impl Serializable for DecimalAccuracy {
    fn serialize(&self) -> Vec<u8> {
        vec![match self {
            DecimalAccuracy::Exact => 1,
            DecimalAccuracy::Approximation => 2,
        }]
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        match bytes.next()? {
            1 => Some(DecimalAccuracy::Exact),
            2 => Some(DecimalAccuracy::Approximation),
            _ => None,
        }
    }
}

impl Serializable for Number {
    fn serialize(&self) -> Vec<u8> {
        match self {
            Number::Decimal(d, a) => {
                let mut result = vec![1];
                result.append(&mut d.serialize().to_vec());
                result.append(&mut a.serialize());
                result
            }

            Self::Rational(numer, denom) => {
                let mut result = vec![2];
                result.append(&mut numer.to_ne_bytes().to_vec());
                result.append(&mut denom.to_ne_bytes().to_vec());
                result
            }
        }
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        let first_byte = bytes.next()?;
        match first_byte {
            1 => {
                let decimal = Decimal::deserialize(
                    bytes.take(16).collect::<Vec<_>>().try_into().ok()?
                );
                let accuracy = DecimalAccuracy::deserialize(bytes)?;
                Some(Number::Decimal(decimal, accuracy))
            }

            2 => {
                let numer: [u8; 8] = bytes.take(8).collect::<Vec<_>>().try_into().ok()?;
                let denom: [u8; 8] = bytes.take(8).collect::<Vec<_>>().try_into().ok()?;
                Some(Number::Rational(
                    i64::from_ne_bytes(numer),
                    i64::from_ne_bytes(denom),
                ))
            }

            _ => None
        }
    }
}
