use core::alloc::Layout;
use core::cmp::{Ordering, max};
use core::mem::{self, Discriminant};
use core::ops::Deref;
use core::str::FromStr;

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::{vec, vec::Vec};
use num_traits::{FromPrimitive, One, Zero};
use rust_decimal::{Decimal};

use crate::Number;
use crate::error::{Error, MathsError};
use crate::node::common;
use crate::decimal_ext::DecimalExtensions;
use crate::render::{Glyph, LayoutBlock, Layoutable, MergeBaseline, Renderer};
use crate::nav::NavPathNavigator;

use super::simplified::{Simplifiable, SimplifiedNode};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum StructuredNode {
    Number(Number),
    Variable(char),
    Sqrt(Box<StructuredNode>),
    Power(Box<StructuredNode>, Box<StructuredNode>),
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
    pub fn disambiguate(&self) -> Result<StructuredNode, MathsError> {
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

            StructuredNode::Number(_) | StructuredNode::Sqrt(_) | StructuredNode::Parentheses(_) | StructuredNode::Variable(_) | StructuredNode::Power(_, _)
                => self.clone(),
        })
    }

    /// Evaluates this node into a single number.
    pub fn evaluate(&self) -> Result<Number, MathsError> {
        match self {
            StructuredNode::Number(n) => Ok((*n).into()),
            StructuredNode::Variable(c) => Err(MathsError::MissingVariable),
            StructuredNode::Sqrt(inner) =>
                inner.evaluate()?.to_decimal().sqrt().map(|x| x.into()).ok_or(MathsError::InvalidSqrt),
            StructuredNode::Power(b, e) =>
                Ok(b.evaluate()?.to_decimal().powd(e.evaluate()?.to_decimal()).into()),
            StructuredNode::Add(a, b) => Ok(a.evaluate()? + b.evaluate()?),
            StructuredNode::Subtract(a, b) => Ok(a.evaluate()? - b.evaluate()?),
            StructuredNode::Multiply(a, b) => Ok(a.evaluate()? * b.evaluate()?),
            StructuredNode::Divide(a, b) => Ok(a.evaluate()?.checked_div(b.evaluate()?)?),
            StructuredNode::Parentheses(inner) => inner.evaluate(),
        }
    }

    /// Walks over all nodes in this tree.
    pub fn walk(&self, func: &impl Fn(&StructuredNode)) {
        func(self);
        match self {
            StructuredNode::Add(l, r)
            | StructuredNode::Subtract(l, r)
            | StructuredNode::Multiply(l, r)
            | StructuredNode::Divide(l, r) => {
                l.walk(func);
                r.walk(func);
            },
            StructuredNode::Sqrt(inner) | StructuredNode::Parentheses(inner) => {
                inner.walk(func);
            },
            StructuredNode::Power(b, e) => {
                b.walk(func);
                e.walk(func);
            }

            StructuredNode::Number(_) | StructuredNode::Variable(_) => (),
        }
    }

    /// Walks over all nodes in this tree, allowing them to be mutated.
    pub fn walk_mut(&mut self, func: &mut impl FnMut(&mut StructuredNode)) {
        func(self);
        match self {
            StructuredNode::Add(l, r)
            | StructuredNode::Subtract(l, r)
            | StructuredNode::Multiply(l, r)
            | StructuredNode::Divide(l, r) => {
                l.walk_mut(func);
                r.walk_mut(func);
            },
            StructuredNode::Sqrt(inner) | StructuredNode::Parentheses(inner) => {
                inner.walk_mut(func);
            },
            StructuredNode::Power(b, e) => {
                b.walk_mut(func);
                e.walk_mut(func);
            }

            StructuredNode::Number(_) | StructuredNode::Variable(_) => (),
        }
    }    

    /// Returns a clone of this node tree where all usages of a variable are replaced with another
    /// set of nodes.
    pub fn substitute_variable(&self, var_name: char, subst: &StructuredNode) -> StructuredNode {
        let mut clone = self.clone();
        clone.walk_mut(&mut |n| {
            if let StructuredNode::Variable(actual_var_name) = n {
                if *actual_var_name == var_name {
                    *n = subst.clone();
                }
            }
        });
        clone
    }
}

/// Calculates layout for a binop, with the operator being the `glyph`.
fn layout_binop(renderer: &mut impl Renderer, glyph: Glyph, left: &StructuredNode, right: &StructuredNode) -> LayoutBlock {
    // These are structured nodes, which (currently) never have a cursor

    let left_layout = left.layout(renderer, None);
    let binop_layout = LayoutBlock::from_glyph(renderer, glyph)
        .move_right_of_other(&left_layout);
    let right_layout = right.layout(renderer, None)
        .move_right_of_other(&binop_layout);

    left_layout
        .merge_along_baseline(&binop_layout)
        .merge_along_baseline(&right_layout)
}

impl Layoutable for StructuredNode {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock {
        match self {
            StructuredNode::Number(Number::Decimal(mut number)) => {
                let negative = number < Decimal::zero();
                if negative {
                    number = -number;
                }

                let mut glyph_layouts = number
                    .to_string()
                    .chars()
                    .map(|c| 
                        if c == '.' {
                            Glyph::Point
                        } else {
                            Glyph::Digit { number: c.to_digit(10).unwrap() as u8 }
                        }
                    )
                    .map(|g| LayoutBlock::from_glyph(renderer, g))
                    .collect::<Vec<_>>();

                if negative {
                    glyph_layouts.insert(
                        0, 
                        LayoutBlock::from_glyph(renderer, Glyph::Subtract)
                    )
                }

                LayoutBlock::layout_horizontal(&glyph_layouts[..])
            },
            StructuredNode::Number(Number::Rational(numer, denom)) => {
                if *denom == 1 {
                    StructuredNode::Number(Number::Decimal(Decimal::from_i64(*numer).unwrap())).layout(renderer, path)
                } else {
                    common::layout_fraction(
                        &StructuredNode::Number(Number::Decimal(Decimal::from_i64(*numer).unwrap())),
                        &StructuredNode::Number(Number::Decimal(Decimal::from_i64(*denom).unwrap())),
                        renderer,
                        None
                    )
                }
            },

            StructuredNode::Variable(v) => LayoutBlock::from_glyph(renderer, Glyph::Variable { name: *v }),

            StructuredNode::Add(left, right) => layout_binop(renderer, Glyph::Add, left, right),
            StructuredNode::Subtract(left, right) => layout_binop(renderer, Glyph::Subtract, left, right),
            StructuredNode::Multiply(left, right) => layout_binop(renderer, Glyph::Multiply, left, right),

            StructuredNode::Divide(top, bottom)
                => common::layout_fraction(top.deref(), bottom.deref(), renderer, path),
            StructuredNode::Sqrt(inner)
                => common::layout_sqrt(inner.deref(), renderer, path),
            StructuredNode::Parentheses(inner)
                => common::layout_parentheses(inner.deref(), renderer, path),
            StructuredNode::Power(base, exp)
                => common::layout_power(Some(base.deref()), exp.deref(), renderer, path),
        }
    }
}

impl Simplifiable for StructuredNode {
    fn simplify(&self) -> SimplifiedNode {
        match self {
            &Self::Number(n) => SimplifiedNode::Number(n),
            &Self::Variable(n) => SimplifiedNode::Variable(n),

            &Self::Add(ref l, ref r) => SimplifiedNode::Add(vec![
                l.simplify(), 
                r.simplify(),
            ]),
            &Self::Subtract(ref l, ref r) => SimplifiedNode::Add(vec![
                l.simplify(), 
                r.simplify().negate(),
            ]),

            &Self::Multiply(ref l, ref r) => SimplifiedNode::Multiply(vec![
                l.simplify(), 
                r.simplify(),
            ]),
            &Self::Divide(ref l, ref r) => SimplifiedNode::Multiply(vec![
                l.simplify(),
                r.simplify().reciprocal(),
            ]),

            Self::Sqrt(n) => SimplifiedNode::Power(
                box n.simplify(),
                box SimplifiedNode::Number(Number::Rational(1, 2)),
            ),
            Self::Power(b, e) => SimplifiedNode::Power(
                box b.simplify(),
                box e.simplify(),
            ),

            Self::Parentheses(n) => n.simplify(),
        }
    }
}
