//! Defines and implements the [Upgradable] trait, for converting to a 
//! [structured](crate::node::structured) node tree.

use alloc::{vec::Vec, boxed::Box};

use crate::{StructuredNode, error::NodeError, UnstructuredNodeList, node::parser, UnstructuredNodeRoot, UnstructuredNode};

/// Implemented by types which can be _upgraded_ - that is, converted into a
/// [structured](crate::node::structured) node tree.
pub trait Upgradable {
    /// Attempts to upgrade this node tree, and returns a [StructuredNode] if it succeeds.
    /// 
    /// Failures will primarily occur due to syntax errors; for example, `3+` would be a valid
    /// unstructured node tree (a pair of two tokens, `3` and `+`), but cannot be encoded as a
    /// structured node tree because it is not a syntactically valid mathematical expression. In
    /// cases like this, a [NodeError] is returned instead.
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
                => Ok(StructuredNode::Sqrt(Box::new(inner.upgrade()?))),

            UnstructuredNode::Parentheses(inner)
                => Ok(StructuredNode::Parentheses(Box::new(inner.upgrade()?))),

            UnstructuredNode::Fraction(a, b)
                => Ok(StructuredNode::Divide(Box::new(a.upgrade()?), Box::new(b.upgrade()?))),

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
