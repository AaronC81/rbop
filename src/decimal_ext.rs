//! Provides one trait, [DecimalExtensions], which adds additional methods to [Decimal].

use num_traits::One;
use rust_decimal::{Decimal, MathematicalOps};

/// This trait, and its implementation on `Decimal`, exist to add extra methods to `Decimal`.
/// Currently, these are:
///   - `to_parts`, to enable access to the raw structure members of the decimal value.
///   - a backport of `powd` from later versions of rust_decimal.
///   - a backport of `powi` from later versions of rust_decimal. This version already has a
///     function called `powi` (which was renamed to `powu`), so here it's called `pows`.
///   - `is_whole`, which checks if a decimal is equal to its floor (i.e. if it's a whole number).
pub trait DecimalExtensions {
    fn to_parts(&self) -> (u32, u32, u32, u32);
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
    fn pows(&self, exp: i64) -> Decimal {
        // For negative exponents we change x^-y into 1 / x^y.
        // Otherwise, we calculate a standard unsigned exponent
        if exp >= 0 {
            return self.powi(exp);
        }

        // Get the unsigned exponent
        let exp = exp.unsigned_abs();
        let pow = self.powi(exp as i64);
        Decimal::one() / pow
    }

    /// Returns true if this decimal is a whole number.
    fn is_whole(&self) -> bool {
        self.floor() == *self
    }
}
