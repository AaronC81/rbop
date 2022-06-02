use rust_decimal::{MathematicalOps, Decimal};

use crate::{Number, error::MathsError};

use super::structured::{EvaluationSettings, AngleUnit};

/// A mathematical function, for which an invocation may appear in an unstructured or structured
/// node tree.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Function {
    Sine,
    Cosine,
}

impl Function {
    /// The suggested text displayed when this function is rendered.
    /// (Renderer implementations are free to ignore this.)
    pub fn render_name(&self) -> &'static str {
        match self {
            Self::Sine => "sin",
            Self::Cosine => "cos",
        }
    }

    /// The number of arguments for this function.
    pub fn argument_count(&self) -> usize {
        match self {
            Self::Sine | Self::Cosine => 1,
        }
    }

    /// Evaluates this function, given values for its arguments. 
    /// Panics if the number of arguments does not equal the expected number. This case shouldn't be
    /// possible for invocations built using rbop's input system, as a fixed number of slots are
    /// created for each argument.
    pub fn evaluate(&self, arguments: &[Number], settings: &EvaluationSettings) -> Result<Number, MathsError> {
        if arguments.len() != self.argument_count() {
            panic!("rbop function {:?} expected {} arguments, but got {}", self, self.argument_count(), arguments.len());
        }

        match self {
            Self::Sine | Self::Cosine => {
                // rust_decimal only lets us sine or cosine by interpreting the input as radians, so
                // do a conversion ourselves first if need be
                let mut target = arguments[0].to_decimal();
                if settings.angle_unit == AngleUnit::Degree {
                    target *= Decimal::PI / Decimal::from(180)
                }

                Ok(Number::Decimal(match self {
                    Self::Sine => target.sin(),
                    Self::Cosine => target.cos(),
                }))
            }
        }
    }
}
