use core::str::FromStr;

use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use rust_decimal::Decimal;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Fraction {
    numerator: Decimal,
    denominator: Decimal,
}

impl Fraction {
    pub fn new(numerator: Decimal, denominator: Decimal) -> Self {
        Self { numerator, denominator }
    }
}

/// Converts a number to an approximate fraction by traversing the Stern-Brocot tree. This code is
/// a translation of this answer: https://stackoverflow.com/a/32903747/2626000
///
/// The `accuracy` parameter must be between 0 and 1. A lower accuracy parameter (closer to 0) will
/// return a closer fractional representation, but take longer to execute.
pub fn decimal_to_fraction(mut value: Decimal, accuracy: Decimal) -> Fraction {
    let sign = if value.is_sign_positive() {
        Decimal::one()
    } else {
        Decimal::from_i8(-1).unwrap()
    };

    value = value.abs();

    let max_error = if value.is_zero() {
        accuracy
    } else {
        value * accuracy
    };

    // Split the whole and decimal parts
    let whole = value.floor();
    value -= whole;
    let whole = whole;

    // If this is already below the accuracy threshold, then great - we've got (basically) a whole
    // number. Return this over 1
    if value < max_error {
        return Fraction::new(
            sign * whole,
            Decimal::one()
        )
    }

    // Check the error on the other side too
    if Decimal::one() - max_error < value {
        return Fraction::new(sign * (whole + Decimal::one()), Decimal::one())
    }

    // Iterate in to find the fraction
    let mut lower = Fraction::new(Decimal::zero(), Decimal::one());
    let mut upper = Fraction::new(Decimal::one(), Decimal::one());

    loop {
        let middle_numerator = lower.numerator + upper.numerator;
        let middle_denominator = lower.denominator + upper.denominator;


        if middle_denominator * (value + max_error) < middle_numerator
        {
            // real + error < middle : middle is our new upper
            upper.numerator = middle_numerator;
            upper.denominator = middle_denominator;
        }
        else if middle_numerator < (value - max_error) * middle_denominator
        {
            // middle < real - error : middle is our new lower
            lower.numerator = middle_numerator;
            lower.denominator = middle_denominator;
        }
        else
        {
            // Middle is our best fraction
            return Fraction::new(
                (whole * middle_denominator + middle_numerator) * sign,
                middle_denominator
            );
        }
    }
}

/// This trait, and its implementation on `Decimal`, exist to add extra methods to `Decimal`.
/// Currently, these are:
///   - `to_parts`, to enable access to the raw structure members of the decimal value.
///   - a backport of `powd` from later versions of rust_decimal.
///   - `is_whole`, which checks if a decimal is equal to its floor (i.e. if it's a whole number).
pub trait DecimalExtensions {
    fn to_parts(&self) -> (u32, u32, u32, u32);
    fn powd(&self, exp: Decimal) -> Decimal;
    fn is_whole(&self) -> bool;
}

impl DecimalExtensions for Decimal {
    // This implementation requires access to the "lo", "mid" and "hi" struct fields.
    // There aren't any methods to do this, but we can serialize and unpack ourselves!
    fn to_parts(&self) -> (u32, u32, u32, u32) {
        let s = self.serialize();
        (
            s[0] as u32 | s[1] as u32 >> 8 | s[2] as u32 >> 16 | s[3] as u32 >> 24,
            s[4] as u32 | s[5] as u32 >> 8 | s[6] as u32 >> 16 | s[7] as u32 >> 24,
            s[8] as u32 | s[9] as u32 >> 8 | s[10] as u32 >> 16 | s[11] as u32 >> 24,
            s[12] as u32 | s[13] as u32 >> 8 | s[14] as u32 >> 16 | s[15] as u32 >> 24,
        )
    }

    fn powd(&self, exp: Decimal) -> Decimal {
        let (_, self_lo, self_mid, self_hi) = self.to_parts();
        let (_, exp_lo,  exp_mid,  exp_hi ) = exp.to_parts();

        if exp.is_zero() {
            return Decimal::one();
        }
        if self.is_zero() {
            return Decimal::zero();
        }
        if self.is_one() {
            return Decimal::one();
        }
        if exp.is_one() {
            return *self;
        }

        // If the scale is 0 then it's a trivial calculation
        let exp = exp.normalize();
        if exp.scale() == 0 {
            if exp_mid != 0 || exp_hi != 0 {
                // Exponent way too big
                panic!("power overflow");
            }

            if exp.is_sign_negative() {
                // TODO: this needs more to be backported, I don't want to do that now
                panic!("negative powers not supported")
            } else {
                return self.powi(exp_lo as u64);
            }
        }

        // We do some approximations since we've got a decimal exponent.
        // For positive bases: a^b = exp(b*ln(a))
        let negative = self.is_sign_negative();
        let e = match self.abs().ln().checked_mul(exp) {
            Some(e) => e,
            None => panic!("power overflow"),
        };
        e.exp();
        let mut result = e.exp();
        result.set_sign_negative(negative);
        result
    }

    /// Returns true if this decimal is a whole number.
    fn is_whole(&self) -> bool {
        self.floor() == *self
    }
}
