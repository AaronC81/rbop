use alloc::boxed::Box;
use rust_decimal::{Decimal, MathematicalOps};

use crate::error::{Error, MathsError};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum StructuredNode {
    Number(Decimal),
    Sqrt(Box<StructuredNode>),
    Add(Box<StructuredNode>, Box<StructuredNode>),
    Subtract(Box<StructuredNode>, Box<StructuredNode>),
    Multiply(Box<StructuredNode>, Box<StructuredNode>),
    Divide(Box<StructuredNode>, Box<StructuredNode>),
    Parentheses(Box<StructuredNode>),
}

impl StructuredNode {
    /// Returns true if this node is `Add` or `Subtract`.
    pub fn add_or_sub(&self) -> bool {
        matches!(&self, StructuredNode::Add(_, _) | StructuredNode::Subtract(_, _))
    }

    /// Returns true if this node is `Multiply` or `Divide`.
    pub fn mul_or_div(&self) -> bool {
        matches!(&self, StructuredNode::Multiply(_, _) | StructuredNode::Divide(_, _))
    }

    /// Returns a clone of this node wrapped in `Parentheses`.
    pub fn in_parentheses(&self) -> StructuredNode {
        StructuredNode::Parentheses(box self.clone())
    }

    /// If `parens` is true, returns a clone of this node wrapped in `Parentheses`, otherwise just
    /// returns a plain clone of this node.
    pub fn in_parentheses_or_clone(&self, parens: bool) -> StructuredNode {
        if parens {
            self.in_parentheses()
        } else {
            self.clone()
        }
    }

    /// Returns a clone of this node tree with added parentheses to show the order of operations
    /// when the tree is rendered.
    /// The tree should be upgraded before doing this.
    pub fn disambiguate(&self) -> Result<StructuredNode, Box<dyn Error>> {
        Ok(match self {
            // We need to add parentheses around:
            //   - operations which mix precedence, e.g. (3+2)*4
            //   - operations which go against standard associativity for - and /, e.g. 3-(3-2)

            StructuredNode::Multiply(l, r) => {
                let l = l.in_parentheses_or_clone(l.add_or_sub());
                let r = r.in_parentheses_or_clone(r.add_or_sub() || r.mul_or_div());
                StructuredNode::Multiply(box l, box r)
            }
            StructuredNode::Divide(l, r) => {
                let l = l.in_parentheses_or_clone(l.add_or_sub());
                let r = r.in_parentheses_or_clone(r.add_or_sub() || r.mul_or_div());
                StructuredNode::Divide(box l, box r)
            }

            StructuredNode::Add(l, r) => {
                let r = r.in_parentheses_or_clone(r.add_or_sub());
                StructuredNode::Add(l.clone(), box r)
            }
            StructuredNode::Subtract(l, r) => {
                let r = r.in_parentheses_or_clone(r.add_or_sub());
                StructuredNode::Subtract(l.clone(), box r)
            }

            StructuredNode::Number(_) | StructuredNode::Sqrt(_) | StructuredNode::Parentheses(_)
                => self.clone(),
        })
    }

    /// Evaluates this node into a single number.
    pub fn evaluate(&self) -> Result<Decimal, Box<dyn Error>> {
        match self {
            StructuredNode::Number(n) => Ok((*n).into()),
            StructuredNode::Sqrt(inner) =>
                inner.evaluate()?.sqrt().ok_or(box MathsError("illegal sqrt".into())),
            StructuredNode::Add(a, b) => Ok(a.evaluate()? + b.evaluate()?),
            StructuredNode::Subtract(a, b) => Ok(a.evaluate()? - b.evaluate()?),
            StructuredNode::Multiply(a, b) => Ok(a.evaluate()? * b.evaluate()?),
            StructuredNode::Divide(a, b) => Ok(a.evaluate()? / b.evaluate()?),
            StructuredNode::Parentheses(inner) => inner.evaluate(),
        }
    }
}
