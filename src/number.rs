use core::{cmp::Ordering, convert::TryInto, ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign}};

use alloc::{vec, vec::Vec, boxed::Box};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use rust_decimal::Decimal;

use crate::{decimal_ext::DecimalExtensions, node::unstructured::Serializable, error::{Error, MathsError}};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Number {
    Decimal(Decimal),
    Rational(i64, i64),
}

impl Number {
    /// Converts this number to a decimal:
    ///   - For `Decimal`, this simply unwraps the variant.
    ///   - For `Rational`, this divides the numerator by the denominator after converting both to
    ///     decimals.
    pub fn to_decimal(&self) -> Decimal {
        match self {
            Number::Decimal(d) => *d,
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
    ///   - For `Decimal`, this does nothing.
    ///   - For `Rational`, this divides the numerator and denominator by their GCD. Also ensures
    ///     that any negative sign is on the numerator, not the denominator.
    fn simplify(&self) -> Number {
        match self {
            Self::Decimal(d) => Self::Decimal(*d),
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
            Self::Decimal(d) => Self::Decimal(Decimal::one() / d),
            Self::Rational(numer, denom) => Self::Rational(*denom, *numer),
        }
    }

    /// If this is a whole number, returns it. Otherwise returns None.
    pub fn to_whole(&self) -> Option<i64> {
        match self {
            Self::Decimal(d)
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

    /// Divides this number by another number, or returns an error if the divisor is zero.
    pub fn checked_div(&self, other: Number) -> Result<Number, MathsError> {
        if other.is_zero() {
            Err(MathsError::DivisionByZero)
        } else {
            Ok(*self / other)
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
        Self::Decimal(d)
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
            Self::Decimal(d) => Self::Decimal(-d),
        }
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if let (l@Self::Rational(_, _), r@Self::Rational(_, _)) = (self, rhs) {
            let (l, r) = l.to_common_with(r);

            Number::Rational(l.numerator() + r.numerator(), l.denominator()).simplify()
        } else {
            Number::Decimal(self.to_decimal() + rhs.to_decimal())
        }
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
        if let (Self::Rational(ln, ld), Self::Rational(rn, rd)) = (self, rhs) {
            Number::Rational(ln * rn, ld * rd).simplify()
        } else {
            Number::Decimal(self.to_decimal() * rhs.to_decimal()).simplify()
        }
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
            Self::Decimal(d) => d.is_zero(),
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
            Self::Decimal(d) => d.is_one(),
            Self::Rational(n, d) => n == d,
        }
    }
}

impl Serializable for Number {
    fn serialize(&self) -> Vec<u8> {
        match self {
            Number::Decimal(d) => {
                let mut result = vec![1];
                result.append(&mut d.serialize().to_vec());
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
                Some(Number::Decimal(Decimal::deserialize(
                    bytes.take(16).collect::<Vec<_>>().try_into().ok()?
                )))
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
