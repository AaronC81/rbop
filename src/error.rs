//! Error types for various operations.

use alloc::{fmt, vec, vec::Vec};

use crate::serialize::Serializable;

/// A trait implemented on any rbop error.
pub trait Error : alloc::fmt::Display + alloc::fmt::Debug {}

/// An error which occurs while parsing or upgrading a node tree.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum NodeError {
    /// The parser was unable to use all of the tokens it was given, indicating a syntax error.
    UnexpectedTokensAtEnd,

    /// A power was used, but while upgrading, there was no suitable node preceding it to use as a
    /// base.
    PowerMissingBase,

    /// The parser was expecting a node which could be parsed as a unit, but could not find one,
    /// indicating a syntax error.
    ExpectedUnit,

    /// While upgrading, a [Token](crate::node::unstructured::UnstructuredNode::Token) was found.
    /// These should always be handled by the parser, so finding one during an upgrade represents an
    /// internal error.
    CannotUpgradeToken,

    /// A numeral used in an expression does not fit into rbop's number representation.
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

/// A mathematical error encountered while evaluating a node tree.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MathsError {
    /// Attempted to divide by zero.
    DivisionByZero,

    /// A square root operation was invalid, as determined by the implementation of
    /// [rust_decimal::MathematicalOps::sqrt].
    InvalidSqrt,

    /// A variable was used in the expression, but variables cannot currently have values, so this
    /// is invalid.
    MissingVariable,

    /// The result of an operation does not fit into rbop's number representation.
    Overflow,

    /// Raising to a power would give an imaginary result, which rbop cannot represent.
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
