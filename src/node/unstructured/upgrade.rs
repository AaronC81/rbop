use alloc::vec::Vec;

use crate::{StructuredNode, error::NodeError, UnstructuredNodeList, node::parser, UnstructuredNodeRoot, UnstructuredNode};

/// Implemented by types which can be _upgraded_ - that is, converted into a `StructuredNode`.
pub trait Upgradable {
    fn upgrade(&self) -> Result<StructuredNode, NodeError>;
}

impl Upgradable for UnstructuredNodeList {
    fn upgrade(&self) -> Result<StructuredNode, NodeError> {
        parser::Parser {
            index: 0,
            nodes: &self.items[..]
        }.parse()
    }
}

impl Upgradable for UnstructuredNodeRoot {
    fn upgrade(&self) -> Result<StructuredNode, NodeError> {
        self.root.upgrade()
    }
}

impl Upgradable for UnstructuredNode {
    fn upgrade(&self) -> Result<StructuredNode, NodeError> {
        match self {
            UnstructuredNode::Sqrt(inner)
                => Ok(StructuredNode::Sqrt(box inner.upgrade()?)),

            UnstructuredNode::Parentheses(inner)
                => Ok(StructuredNode::Parentheses(box inner.upgrade()?)),

            UnstructuredNode::Fraction(a, b)
                => Ok(StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?)),

            // Parser should always handle this
            UnstructuredNode::Power(_)
                => Err(NodeError::PowerMissingBase),

            UnstructuredNode::FunctionCall(func, args)
                => Ok(StructuredNode::FunctionCall(*func, 
                    args.iter().map(|a| a.upgrade()).collect::<Result<Vec<_>, _>>()?
                )),

            UnstructuredNode::Token(_) => Err(NodeError::CannotUpgradeToken),
        }
    }
}
