//! Mathematical functions which can be used by all kinds of node tree, and called during
//! evaluation.

use alloc::{vec::Vec, vec};
use num_integer::Integer;
use num_traits::{ToPrimitive, FromPrimitive};
use rust_decimal::{MathematicalOps, Decimal};

use crate::{Number, error::MathsError, number::DecimalAccuracy, serialize::Serializable};

use super::{structured::{EvaluationSettings, AngleUnit}};

/// A mathematical function, for which an invocation may appear in an unstructured or structured
/// node tree.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Function {
    Sine,
    Cosine,
    GreatestCommonDenominator,
}

impl Function {
    /// The suggested text displayed when this function is rendered.
    /// (Renderer implementations are free to ignore this.)
    pub fn render_name(&self) -> &'static str {
        match self {
            Self::Sine => "sin",
            Self::Cosine => "cos",
            Self::GreatestCommonDenominator => "gcd",
        }
    }

    /// The number of arguments for this function.
    pub fn argument_count(&self) -> usize {
        match self {
            Self::Sine | Self::Cosine => 1,
            Self::GreatestCommonDenominator => 2,
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

                if settings.use_floats && let Some(float) = target.to_f32() {
                    Ok(Number::Decimal(Decimal::from_f32(match self {
                        Self::Sine => libm::sinf(float),
                        Self::Cosine => libm::cosf(float),
                        _ => unreachable!()
                    }).unwrap(), DecimalAccuracy::Approximation))
                } else {
                    Ok(Number::Decimal(match self {
                        Self::Sine => target.sin(),
                        Self::Cosine => target.cos(),
                        _ => unreachable!()
                    }, DecimalAccuracy::Approximation))
                }
            },

            Self::GreatestCommonDenominator => {
                // This is an integer operation, so convert both numbers to integers - if we can't,
                // just return 1
                let int_a = if let Some(x) = arguments[0].to_whole() { x } else {
                    return Ok(Number::Decimal(Decimal::ONE, DecimalAccuracy::Exact))
                };
                let int_b = if let Some(x) = arguments[1].to_whole() { x } else {
                    return Ok(Number::Decimal(Decimal::ONE, DecimalAccuracy::Exact))
                };

                Ok(int_a.gcd(&int_b).into())
            }
        }
    }
}

impl Serializable for Function {
    fn serialize(&self) -> Vec<u8> {
        vec![match self {
            Function::Sine => 1,
            Function::Cosine => 2,
            Function::GreatestCommonDenominator => 3,
        }]
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        match bytes.next() {
            Some(1) => Some(Function::Sine),
            Some(2) => Some(Function::Cosine),
            Some(3) => Some(Function::GreatestCommonDenominator),

            _ => None,
        }
    }
}
