use alloc::{string::String, fmt};

pub trait Error : alloc::fmt::Display + alloc::fmt::Debug {}

#[derive(Debug, Clone)]
pub struct NodeError(pub String);

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
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
