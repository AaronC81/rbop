//! An abstraction for something which can be evaluated into a number.
//! 
//! Currently, this is only implemented for [StructuredNode](crate::StructuredNode). This exists so 
//! that other evaluable kinds of nodes could be added later, for example compiled nodes which 
//! aren't editable but are much faster to evaluate.

use crate::{Number, error::MathsError};

/// Something which can be evaluated into a number, optionally with variables substituted into it.
pub trait Evaluable {
    /// The type returned by [substitute](#method.substitute). This is likely to be `Self`, but the
    /// option exists to choose a different type, in case it is possible to optimise for the case
    /// where evaluation is faster if no substitutions need to take place.
    type Substituted: Evaluable;
    
    /// A type containing settings needed for evaluation.
    type Settings;

    /// Evaluates this expression and returns either a [Number] with the result, or a [MathsError]
    /// if evaluation was not successful.
    fn evaluate(self, settings: &Self::Settings) -> Result<Number, MathsError>;

    /// Substitutes the named variable with a given value, and returns a new evaluable expression.
    fn substitute(self, variable: char, value: Number) -> Self::Substituted;
}
