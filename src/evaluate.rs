use crate::{Number, error::MathsError};

/// Something which can be evaluated into a number, optionally with variables substituted into it.
pub trait Evaluable {
    type Substituted: Evaluable;
    type Settings;

    fn evaluate(self, settings: &Self::Settings) -> Result<Number, MathsError>;
    fn substitute(self, variable: char, value: Number) -> Self::Substituted;
}
