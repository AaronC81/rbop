// This module sets out a simplified node tree which contains fewer variants than `StructeredNode`.
// For example, 1 - 2 is just 1 + (-1 * 2), so there is no need for a subtraction node. This makes
// it easier to perform simplification passes on the tree.
//
// This is very heavily inspired by the system used by Poincaré, the mathematics system in NumWorks'
// Epsilon. The fantastic people at NumWorks have written some really nice documentation about the
// inner workings of Poincaré: https://www.numworks.com/resources/engineering/software/poincare/
// (The ways that rbop and Poincaré do things are actually quite similar, so this would've been a
// handy page to find earlier!)

use core::{cmp::Ordering, mem};

use alloc::{boxed::Box, vec, vec::Vec};
use num_traits::{One, Zero};
use rust_decimal::MathematicalOps;

use crate::{Number, error::MathsError};

use super::function::Function;

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
    FunctionCall(Function, Vec<SimplifiedNode>),
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
            SimplifiedNode::FunctionCall(_, args) => {
                for arg in args {
                    arg.sort();
                }
            }
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
            SimplifiedNode::Power(_, _) | Self::Number(_) | Self::Variable(_) | Self::FunctionCall(_, _) => (),
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
            Self::FunctionCall(func, args) => Self::FunctionCall(
                func,
                args.into_iter().map(|n| n.flatten()).collect(),
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

                // Is the power a rational number, with a non-one numerator and denominator?
                //   e.g. 3/2, but not 2 or 1/2
                if let box SimplifiedNode::Number(Number::Rational(n, d)) = e {
                    if *n != 1 && *d != 1 {
                        // Split up this power - a^(3/2) = (a^3)^1/2
                        let new_inner_exponent = SimplifiedNode::Number(Number::Rational(*n, 1));
                        let new_outer_exponent = SimplifiedNode::Number(Number::Rational(1, *d));

                        *self = SimplifiedNode::Power(
                            box SimplifiedNode::Power(
                                b.clone(),
                                box new_inner_exponent,
                            ),
                            box new_outer_exponent,
                        );

                        // Restart the reduction for this node - this shouldn't recurse infinitely,
                        // due to our != 1 barrier
                        self.reduce()?;
                        return Ok(PerformedReduction)
                    }
                }

                // There are some special cases with integer exponents, handle them here
                if let box SimplifiedNode::Number(n) = e {
                    // Anything to the power 1 doesn't change, so we can remove the power
                    if let Some(1) = n.to_whole() {
                        *self = b.as_ref().clone();
                        return Ok(PerformedReduction)
                    }

                    // Anything to the power 0 becomes 1
                    if let Some(0) = n.to_whole() {
                        *self = SimplifiedNode::Number(Number::one());
                        return Ok(PerformedReduction)
                    }
                }

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
                            // What kind?
                            if let Some(exp) = exp.to_whole() {
                                // If the exponent is a whole number, we might as well raise, since
                                // no accuracy would be lost
                                *self = SimplifiedNode::Number(base.powi(exp));
                                status = PerformedReduction;
                            } else if let Number::Decimal(exp) = exp {
                                // If the exponent is a non-whole decimal, precision loss is
                                // inevitable anyway, so power now!
                                *self = SimplifiedNode::Number(Number::Decimal(
                                    base.to_decimal().powd(*exp)
                                ));
                                status = PerformedReduction;
                            } else if let Number::Rational(n, d) = exp {
                                // Only other case is that the exponent is a non-whole rational with
                                // a numerator of 1, since we split up rational powers earlier.
                                // Such exponents represent roots, so leave them as-is.
                                if *n != 1 || *d == 1 {
                                    unreachable!();
                                }
                            } else {
                                unreachable!();
                            }
                        } else {
                            // The exponent isn't reduced to a number, so we can't raise to it!

                            // TODO: When approximation implemented, if base is decimal, consider
                            // approximating the exponent and reducing to the powered result
                        }
                    },

                    box SimplifiedNode::Power(inner_base, inner_exp) => {
                        // So we're powering a power - (x^a)^b = x^(ab), so we can multiply and
                        // reduce the two exponents to form a new power node
                        //
                        // (The inner base and exponent were already reduced recursively earlier)
                        
                        let mut new_exp = SimplifiedNode::Multiply(vec![
                            inner_exp.as_ref().clone(),
                            e.as_ref().clone(),
                        ]);
                        new_exp.reduce()?; 
                        
                        *self = SimplifiedNode::Power(
                            inner_base.clone(),
                            box new_exp,
                        );
                        status = PerformedReduction
                    }

                    box SimplifiedNode::Multiply(v) => {
                        // To raise a series of multiplications to a power, we can apply that power
                        // to all of the terms of the multiplication instead: (ab)^n = a^n*b^n
                        //
                        // Construct a new multiplication node where we do this
                        let mut new_terms = vec![];

                        for term in v {
                            new_terms.push(SimplifiedNode::Power(
                                box term.clone(),
                                e.clone(),
                            ));
                        }

                        *self = SimplifiedNode::Multiply(new_terms);
                        self.reduce()?;

                        status = PerformedReduction
                    }
                        
                    box SimplifiedNode::Add(_) => todo!(),      // TODO: Expand

                    box SimplifiedNode::FunctionCall(_, _) => todo!(), // TODO
                }
            }

            SimplifiedNode::Multiply(v) => {
                // Sort children
                v.sort();

                // Reduce children
                Self::reduce_vec(v)?;

                // Are there numbers at the start?
                if let Some(numbers) = Self::collect_numbers_from_start(&v[..]) {
                    let numbers_len = numbers.len();

                    // Are any of numbers 0? If so, this ENTIRE multiplication node evaluates to 0
                    if numbers.iter().any(|n| n.is_zero()) {
                        *self = Self::Number(Number::zero());
                        return Ok(PerformedReduction)
                    }

                    // Multiply all of these together
                    let result = numbers.iter().fold(Number::one(), |a, b| a * **b);

                    // Delete the multiplied nodes
                    v.drain(0..numbers_len);

                    // Insert this onto the beginning, unless it's 1, in which case it has no effect
                    // on multiplication
                    if !result.is_one() {
                        v.insert(0, Self::Number(result));
                    }

                    status = PerformedReduction
                }

                // Combine like terms, re-reducing if any changed
                if Self::combine_terms(
                    v,
                    |n|
                        if let SimplifiedNode::Power(node, box SimplifiedNode::Number(exp)) = n {
                            Ok((node.as_ref().clone(), *exp))
                        } else {
                            Ok((n.clone(), Number::one()))
                        },
                    |n, c|
                        Ok(SimplifiedNode::Power(box n.clone(), box SimplifiedNode::Number(c)))
                )? == PerformedReduction {
                    self.reduce()?;
                    return Ok(PerformedReduction)
                };

                // If there is only one child, reduce to that child
                if v.len() == 1 {
                    *self = v[0].clone();
                    status = PerformedReduction;
                }
            }

            SimplifiedNode::Add(v) => {
                // Sort children
                v.sort();

                // Reduce children
                Self::reduce_vec(v)?;

                // Are there numbers at the start?
                if let Some(numbers) = Self::collect_numbers_from_start(&v[..]) {
                    let numbers_len = numbers.len();

                    // Add all of these together
                    let result = numbers.iter().fold(Number::zero(), |a, b| a + **b);

                    // Delete the added nodes and insert this onto the beginning
                    v.drain(0..numbers_len);

                    // Insert this onto the beginning, unless it's 0, in which case it has no effect
                    // on addition
                    if !result.is_zero() {
                        v.insert(0, Self::Number(result));
                    }

                    status = PerformedReduction
                }

                // Combine like terms, re-reducing if any changed
                if Self::combine_terms(
                    v,
                    |n|
                        match n {
                            // Due to sorting, the number will always be at the start if there is one
                            // (This is all a pattern! The if_let_guards feature lets us do this.)
                            SimplifiedNode::Multiply(v)
                            if let Some(SimplifiedNode::Number(n)) = v.first() => {
                                // Construct a new multiply out of the non-number nodes
                                let mut result = SimplifiedNode::Multiply(v[1..].to_vec());
                                result.reduce()?;
                                Ok((result, *n))
                            },

                            _ => Ok((n.clone(), Number::one()))
                        },
                    |n, c|
                        Ok(SimplifiedNode::Multiply(vec![
                            SimplifiedNode::Number(c), n
                        ]))
                )? == PerformedReduction {
                    self.reduce()?;
                    return Ok(PerformedReduction)
                };

                // If there is only one child, reduce to that child
                if v.len() == 1 {
                    *self = v[0].clone();
                }
            }
        
            // TODO: how to approach this? Maybe evaluate if all arguments have been reduced to 
            // numbers?
            SimplifiedNode::FunctionCall(_, _) => todo!(),
        }

        Ok(status)
    }

    fn combine_terms(
        vec: &mut Vec<SimplifiedNode>,
        dissect: impl Fn(&SimplifiedNode) -> Result<(SimplifiedNode, Number), MathsError>,
        combine: impl Fn(SimplifiedNode, Number) -> Result<SimplifiedNode, MathsError>,
    ) -> ReductionResult
    {
        // It is assumed that the vec has items, bail if it doesn't
        if vec.is_empty() {
            return Ok(ReductionStatus::NoReduction)
        }

        // TODO: probably not very optimised at all
        let mut combined_any = false;

        // Dissect each item in the vec into its base node and term count
        let mut dissected = vec.iter()
            .map(|x| dissect(x))
            .collect::<Result<Vec<_>, _>>()?;

        // Sort this list by the base nodes, so that equal ones will be adjacent
        dissected.sort_by(|(ln, _), (rn, _)| ln.cmp(rn));

        // Find runs of equal elements
        let mut result = vec![];
        let (mut run_node, mut run_term_count) = dissected[0].clone();
        let mut run_length = 1;
        for (i, (this_node, this_term_count)) in dissected[1..].iter().enumerate() {
            if *this_node == run_node {
                // Keep going with this run!
                run_term_count = run_term_count + *this_term_count;
                run_length += 1;
            } else {
                // Add the run onto the result vec
                if run_length > 1 {
                    result.push(combine(run_node, run_term_count)?);
                    combined_any = true;
                } else {
                    result.push(vec[i].clone());
                }
                
                // Start a new run
                run_node = this_node.clone();
                run_term_count = *this_term_count;
                run_length = 1;
            }
        }

        // Handle end of final run
        if run_length > 1 {
            result.push(combine(run_node, run_term_count)?);
            combined_any = true;
        } else {
            result.push(vec.last().unwrap().clone());
        }

        // Assign result
        *vec = result;
        Ok(if combined_any { ReductionStatus::PerformedReduction } else { ReductionStatus::NoReduction })
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
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ReductionStatus {
    PerformedReduction,
    NoReduction,
}

pub type ReductionResult = Result<ReductionStatus, MathsError>;

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
