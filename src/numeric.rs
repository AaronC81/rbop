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
