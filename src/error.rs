use alloc::{fmt, vec, vec::Vec};

use crate::node::unstructured::Serializable;

pub trait Error : alloc::fmt::Display + alloc::fmt::Debug {}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum NodeError {
    UnexpectedTokensAtEnd,
    PowerMissingBase,
    ExpectedUnit,
    CannotUpgradeToken,
    Overflow,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            NodeError::UnexpectedTokensAtEnd => "syntax error",
            NodeError::PowerMissingBase => "no base given for power",
            NodeError::ExpectedUnit => "syntax error",
            NodeError::CannotUpgradeToken => "internal syntax error",
            NodeError::Overflow => "numeric overflow",
        })
    }
}
impl Error for NodeError {}

impl Serializable for NodeError {
    fn serialize(&self) -> Vec<u8> {
        vec![match self {
            NodeError::UnexpectedTokensAtEnd => 1,
            NodeError::PowerMissingBase => 2,
            NodeError::ExpectedUnit => 3,
            NodeError::CannotUpgradeToken => 4,
            NodeError::Overflow => 5,
        }]
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        Some(match bytes.next()? {
            1 => NodeError::UnexpectedTokensAtEnd,
            2 => NodeError::PowerMissingBase,
            3 => NodeError::ExpectedUnit,
            4 => NodeError::CannotUpgradeToken,
            5 => NodeError::Overflow,

            _ => return None,
        })
    }
}


#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MathsError {
    DivisionByZero,
    InvalidSqrt,
    MissingVariable,
    Overflow,
    Imaginary,
}

impl fmt::Display for MathsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            MathsError::DivisionByZero => "division by zero",
            MathsError::InvalidSqrt => "invalid square root",
            MathsError::MissingVariable => "cannot evaluate variable",
            MathsError::Overflow => "numeric overflow",
            MathsError::Imaginary => "imaginary",
        })
    }
}
impl Error for MathsError {}

impl Serializable for MathsError {
    fn serialize(&self) -> Vec<u8> {
        vec![match self {
            MathsError::DivisionByZero => 1,
            MathsError::InvalidSqrt => 2,
            MathsError::MissingVariable => 3,
            MathsError::Overflow => 4,
            MathsError::Imaginary => 5,
        }]
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        Some(match bytes.next()? {
            1 => MathsError::DivisionByZero,
            2 => MathsError::InvalidSqrt,
            3 => MathsError::MissingVariable,
            4 => MathsError::Overflow,
            5 => MathsError::Imaginary,

            _ => return None,
        })
    }
}
