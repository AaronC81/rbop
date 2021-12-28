use alloc::{string::String, fmt};

pub trait Error : alloc::fmt::Display + alloc::fmt::Debug {}

#[derive(Debug, Clone)]
pub enum NodeError {
    UnexpectedTokensAtEnd,
    PowerMissingBase,
    ExpectedUnit,
    CannotUpgradeToken,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            NodeError::UnexpectedTokensAtEnd => "unexpected tokens at end of input",
            NodeError::PowerMissingBase => "no base given for power",
            NodeError::ExpectedUnit => "expected a unit",
            NodeError::CannotUpgradeToken => "token cannot be upgraded",
        })
    }
}
impl Error for NodeError {}

#[derive(Debug, Clone)]
pub struct MathsError(pub String);

impl fmt::Display for MathsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for MathsError {}
