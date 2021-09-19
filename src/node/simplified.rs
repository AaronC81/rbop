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
use num_traits::{One, Zero};
use rust_decimal::Decimal;

use crate::{Number, error::Error, decimal_ext::DecimalExtensions};

#[derive(Eq, PartialEq, Debug, Clone)]
/// A simplified variant of `StructuredNode`. By "simplified", we mean fewer possible variants which
/// have the same semantic meaning. This provides an easier platform for performing mathematical
/// reduction on a node tree.
pub enum SimplifiedNode {
    Number(Number),
    Variable(char),
    Multiply(Vec<SimplifiedNode>),
    Power(Box<SimplifiedNode>, Box<SimplifiedNode>),
    Add(Vec<SimplifiedNode>),
}

impl SimplifiedNode {
    /// Returns a new node: a multiplication of this node by -1.
    pub fn negate(self) -> SimplifiedNode {
        Self::Multiply(vec![Self::Number(-Number::one()), self])
    }

    /// Returns a new node: this node raised to the power -1.
    pub fn reciprocal(self) -> SimplifiedNode {
        Self::Power(box self, box Self::Number(-Number::one()))
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

    /// Sorts just this level of the node tree. Child items are not recursed into. This can be an
    /// optimization if you have inserted new items into an Add or Multiply which you know are
    /// themselves already sorted, and just wish to re-sort the container.
    ///
    /// Returns &mut self to allow method chaining.
    pub fn sort_one_level(&mut self) -> &mut Self {
        match self {
            SimplifiedNode::Add(n) | SimplifiedNode::Multiply(n) => n.sort(),
            SimplifiedNode::Power(_, _) | Self::Number(_) | Self::Variable(_) => (),
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

    /// Performs a mathematical reduction on this node tree. The resulting tree has the same
    /// semantic meaning as the original tree, aiming for no loss in precision whatsoever, within
    /// the margins of what `Decimal` can represent.
    ///
    /// This node tree MUST be sorted with `sort` first.
    ///
    /// Returns a `ReductionResult` encapsulating:
    ///   - Whether any reduction took place
    ///   - If an error occured during reduction
    pub fn reduce(&mut self) -> ReductionResult {
        use ReductionStatus::*;

        let mut status = NoReduction;
        /*
        Reduction:
        - Run a pass
        - If it finds something, replace reduced nodes, and run another induction pass on the node which
            contained the reduced ones
        - Repeat until pass makes no changes
        */

        match self {
            // There's no reduction which can be done on leaf nodes
            Self::Variable(_) => (),
            Self::Number(_) => (),

            Self::Power(b, e) => {
                // Reduce the base and exponent first
                b.reduce()?;
                e.reduce()?;

                // What's the base which we're raising to a power?
                match b {
                    // Variables can't be raised to a power because we don't know what they are, so 
                    // no reduction can be done here.
                    box SimplifiedNode::Variable(_) => (),

                    box SimplifiedNode::Number(base) => {
                        // Technically a number could always be raised to the power here, but it
                        // kind of depends what the base and exponent are.

                        // Is the exponent a number?
                        if let box SimplifiedNode::Number(exp) = e {
                            // OK, so we _can_ raise to it. If it's a whole number, we might as
                            // well, since no accuracy would be lost
                            if let Some(exp) = exp.to_whole() {
                                *self = SimplifiedNode::Number(base.powi(exp));
                            } else {
                                // TODO: probably keep as root? maybe split into power now and root later? figure out
                                todo!();
                            }
                        } else {
                            // The exponent isn't reduced to a number, so we can't raise to it!
                        }
                    },

                    box SimplifiedNode::Multiply(_) => todo!(), // TODO: Power each child
                    box SimplifiedNode::Power(_, _) => todo!(), // TODO: Merge powers
                    box SimplifiedNode::Add(_) => todo!(),      // TODO: Expand
                }
            }

            SimplifiedNode::Multiply(v) => {
                // Reduce children
                Self::reduce_vec(v)?;

                // Are there numbers at the start?
                if let Some(numbers) = Self::collect_numbers_from_start(&v[..]) {
                    // Are any of numbers 0? If so, this ENTIRE multiplication node evaluates to 0
                    if numbers.iter().any(|n| n.is_zero()) {
                        *self = Self::Number(Number::zero());
                        return Ok(PerformedReduction)
                    }

                    // Multiply all of these together
                    let result = numbers.iter().fold(Number::one(), |a, b| a * **b);

                    // Delete the multiplied nodes and insert this onto the beginning
                    v.drain(0..v.len());
                    v.insert(0, Self::Number(result));

                    status = PerformedReduction

                }

                // If there is only one child, reduce to that child
                if v.len() == 1 {
                    *self = v[0].clone();
                }
            }

            SimplifiedNode::Add(v) => {
                // Reduce children
                Self::reduce_vec(v)?;

                // Are there numbers at the start?
                if let Some(numbers) = Self::collect_numbers_from_start(&v[..]) {
                    // Add all of these together
                    let result = numbers.iter().fold(Number::zero(), |a, b| a + **b);

                    // Delete the added nodes and insert this onto the beginning
                    v.drain(0..v.len());
                    v.insert(0, Self::Number(result));

                    status = PerformedReduction

                }

                // If there is only one child, reduce to that child
                if v.len() == 1 {
                    *self = v[0].clone();
                }
            }
        }

        Ok(status)
    }

    /// Reduces a vec of nodes, and re-sorts the vec if any of the reductions changed a child node.
    fn reduce_vec(vec: &mut Vec<SimplifiedNode>) -> ReductionResult {
        // Reduce all child items, collecting whether any were actually reduced
        let mut any_children_reduced = false;
        for child in vec.iter_mut() {
            if child.reduce()? == ReductionStatus::PerformedReduction {
                any_children_reduced = true;
            }
        }

        // If any child was reduced, re-sort
        if any_children_reduced {
            vec.sort();
        }

        Ok(if any_children_reduced {
            ReductionStatus::PerformedReduction
        } else { 
            ReductionStatus::NoReduction
        })
    }
    
    /// Collects numbers from the beginning of a series of nodes. If there are no numbers at the 
    /// start, returns None.
    fn collect_numbers_from_start(vec: &[SimplifiedNode]) -> Option<Vec<&Number>> {
        // Are there numbers at the start?
        if let Some(Self::Number(first_n)) = vec.get(0) {
            // Yep! Collect all of the numbers
            let mut numbers = vec![first_n];
            let mut i = 1;
            while let Some(Self::Number(n)) = vec.get(i) {
                numbers.push(n);
                i += 1;
            }

            Some(numbers)
        } else {
            None
        }
    }

    fn is_leaf(&self) -> bool {
        matches!(self, Self::Number(_) | Self::Variable(_))
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ReductionStatus {
    PerformedReduction,
    NoReduction,
}

pub type ReductionResult = Result<ReductionStatus, Box<dyn Error>>;

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
