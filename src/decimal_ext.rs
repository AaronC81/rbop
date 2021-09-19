use core::str::FromStr;

use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use rust_decimal::Decimal;

/// This trait, and its implementation on `Decimal`, exist to add extra methods to `Decimal`.
/// Currently, these are:
///   - `to_parts`, to enable access to the raw structure members of the decimal value.
///   - a backport of `powd` from later versions of rust_decimal.
///   - a backport of `powi` from later versions of rust_decimal. This version already has a
///     function called `powi` (which was renamed to `powu`), so here it's called `pows`.
///   - `is_whole`, which checks if a decimal is equal to its floor (i.e. if it's a whole number).
pub trait DecimalExtensions {
    fn to_parts(&self) -> (u32, u32, u32, u32);
    fn powd(&self, exp: Decimal) -> Decimal;
    fn pows(&self, exp: i64) -> Decimal;
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
                return self.pows(-(exp_lo as i64));
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

    fn pows(&self, exp: i64) -> Decimal {
        // For negative exponents we change x^-y into 1 / x^y.
        // Otherwise, we calculate a standard unsigned exponent
        if exp >= 0 {
            return self.powi(exp as u64);
        }

        // Get the unsigned exponent
        let exp = exp.unsigned_abs();
        let pow = self.powi(exp);
        Decimal::one() / pow
    }

    /// Returns true if this decimal is a whole number.
    fn is_whole(&self) -> bool {
        self.floor() == *self
    }
}
