// This module sets out a simplified node tree which contains fewer variants than `StructeredNode`.
// For example, 1 - 2 is just 1 + (-1 * 2), so there is no need for a subtraction node. This makes
// it easier to perform simplification passes on the tree.
//
// This is very heavily inspired by the system used by Poincaré, the mathematics system in NumWorks'
// Epsilon. The fantastic people at NumWorks have written some really nice documentation about the
// inner workings of Poincaré: https://www.numworks.com/resources/engineering/software/poincare/
// (The ways that rbop and Poincaré do things are actually quite similar, so this would've been a
// handy page to find earlier!)

use core::{cmp::Ordering, mem::{self, Discriminant}};

use alloc::{boxed::Box, vec, vec::Vec};
use num_traits::One;
use rust_decimal::Decimal;

#[derive(Eq, PartialEq, Debug, Clone)]
/// A simplified variant of `StructuredNode`. By "simplified", we mean fewer possible variants which
/// have the same semantic meaning. This provides an easier platform for performing mathematical
/// reduction on a node tree.
pub enum SimplifiedNode {
    Number(Decimal),
    Variable(char),
    Multiply(Vec<SimplifiedNode>),
    Power(Box<SimplifiedNode>, Box<SimplifiedNode>),
    Add(Vec<SimplifiedNode>),
}

impl SimplifiedNode {
    /// Returns a new node: a multiplication of this node by -1.
    pub fn negate(self) -> SimplifiedNode {
        Self::Multiply(vec![Self::Number(-Decimal::one()), self])
    }

    /// Returns a new node: this node raised to the power -1.
    pub fn reciprocal(self) -> SimplifiedNode {
        Self::Power(box self, box Self::Number(-Decimal::one()))
    }

    /// Sorts the entire node tree, and returns &mut self to allow method chaining.
    pub fn sort(&mut self) -> &mut Self {
        match self {
            SimplifiedNode::Add(n) | SimplifiedNode::Multiply(n) => {
                n.sort();
                n.iter_mut().for_each(|x| { x.sort(); });
            },
            SimplifiedNode::Power(b, e) => {
                b.sort();
                e.sort();
            },
            Self::Number(_) | Self::Variable(_) => (),
        }

        self
    }

    /// Converts nested `Add` and `Multiply` nodes into a single node, recursively through the whole
    /// node tree.
    ///
    /// For example, the node representation of 1 + (2 + (3 + 4)) + 5 would be converted to simply
    /// 1 + 2 + 3 + 4 + 5, which is equivalent.
    pub fn flatten(self) -> SimplifiedNode {
        match self {
            Self::Add(_) => Self::Add(self.flatten_children()),
            Self::Multiply(_) => Self::Multiply(self.flatten_children()),
            Self::Power(b, e) => Self::Power(
                box b.flatten(),
                box e.flatten()
            ),
            Self::Number(_) | Self::Variable(_) => self
        }
    }

    /// Implementation helper of `flatten`. Can only be called on nodes which have a Vec of
    /// children, currently `Add` and `Multiply`.
    fn flatten_children(self) -> Vec<SimplifiedNode> {
        let mut result = vec![];
        let this_discriminant = mem::discriminant(&self);

        if let Self::Add(items) | Self::Multiply(items) = self {
            for item in items {
                let flattened_item = item.flatten();

                // Both add and multiply are commutative, so we can remove the brackets from 1 + (2
                // + 3) + 4. This `if` statement checks if the flattened child node is of the same
                // type as this one - if so, we can insert its children directly into this node.
                if mem::discriminant(&flattened_item) == this_discriminant {
                    if let Self::Add(mut child_items) | Self::Multiply(mut child_items) = flattened_item {
                        result.append(&mut child_items);
                    } else {
                        unreachable!()
                    }
                } else {
                    result.push(flattened_item);
                }
            }
        } else {
            unreachable!()
        }

        result
    }

    fn is_leaf(&self) -> bool {
        matches!(self, Self::Number(_) | Self::Variable(_))
    }
}

pub trait Simplifiable {
    /// Converts this node into a `SimplifiedNode` tree.
    ///
    /// This operation in itself will not actually perform any "simplification" beyond this
    /// conversion; the caller can use methods on `SimplifiedNode` to do this.
    fn simplify(&self) -> SimplifiedNode;
}

impl PartialOrd for SimplifiedNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimplifiedNode {
    /// Orders nodes based on their type and, if they have one, their value.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // === Equal types =====================================================================
            
            // Compare leaves by comparing their inners
            (&Self::Number(ref l), &Self::Number(ref r)) => l.cmp(r),
            (&Self::Variable(ref l), &Self::Variable(ref r)) => l.cmp(r),

            // Compare sequences (of the same type) by comparing their elements
            (&Self::Add(ref l), &Self::Add(ref r))
            | (&Self::Multiply(ref l), &Self::Multiply(ref r))
                => l.cmp(r),
            
            // Compare powers by first comparing base, then exponent
            (&Self::Power(ref lb, ref le), &Self::Power(ref rb, ref re))
                => lb.cmp(rb).then(le.cmp(re)),

            // === Different types =================================================================

            // Failing all else, use enum definition order
            // (This is what the derivation for *Ord does)
            // mem::discriminant does not implement Ord, so we have to use the intrinsics here :(
            _ => core::intrinsics::discriminant_value(self).cmp(
                &core::intrinsics::discriminant_value(other)
            ),
        }
    }
}
