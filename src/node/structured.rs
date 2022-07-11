//! The structured node tree, suited to evaluation.
//! 
//! If using rbop as a library for mathematical user input, then it is likely that you will want to
//! construct an [unstructured](crate::node::unstructured) node tree first, and then
//! [upgrade](crate::node::unstructured::Upgradable) to a structured node tree.
//! 
//! If using rbop as a mathematics utility library, then it is possible to construct structured
//! nodes directly.

use core::fmt::Display;
use core::ops::Deref;

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::{vec, vec::Vec};
use num_traits::{FromPrimitive, Zero};
use rust_decimal::{Decimal, MathematicalOps};

use crate::Number;
use crate::error::MathsError;
use crate::node::common;
use crate::number::DecimalAccuracy;
use crate::render::{Glyph, LayoutBlock, Layoutable, Renderer, LayoutComputationProperties};
use crate::nav::NavPathNavigator;

use super::function::Function;
use super::simplified::{Simplifiable, SimplifiedNode};

/// An structured node. See the [module-level documentation](crate::node::structured) for more
/// information.
/// 
/// Note that structured nodes are two-operand only; `3+2+4` may be encoded as `Add(Add(3, 2), 4)`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum StructuredNode {
    /// A constant number.
    Number(Number),

    // A variable, identified by a character.
    Variable(char),

    /// A square root applied to other structured nodes.
    Sqrt(Box<StructuredNode>),

    /// A power, with both a base and exponent as structured nodes.
    Power(Box<StructuredNode>, Box<StructuredNode>),

    /// A two-operand addition of two structured nodes.
    Add(Box<StructuredNode>, Box<StructuredNode>),

    /// A two-operand subtraction of two structured nodes.
    Subtract(Box<StructuredNode>, Box<StructuredNode>),

    /// A two-operand multiplication of two structured nodes.
    Multiply(Box<StructuredNode>, Box<StructuredNode>),

    /// A two-operand division of two structured nodes.
    Divide(Box<StructuredNode>, Box<StructuredNode>),

    /// Structured nodes enclosed in parentheses.
    /// 
    /// As structured nodes are rigidly-structured anyway, this does not affect evaluation, but may
    /// be desirable for rendering.
    Parentheses(Box<StructuredNode>),

    /// A function call, with a sequence of arguments passed as structured nodes.
    FunctionCall(Function, Vec<StructuredNode>),
}

/// A unit in which angles are measured.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum AngleUnit {
    Degree,
    Radian,
}

impl Default for AngleUnit {
    fn default() -> Self {
        Self::Degree
    }
}

impl Display for AngleUnit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Degree => "Degree",
            Self::Radian => "Radian",
        })
    }
}

/// Settings for how structured nodes are evaluated into a number.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct EvaluationSettings {
    /// The angle unit to use for trigonometric functions.
    pub angle_unit: AngleUnit,

    /// If true, expensive operations such as trigonometric functions will be evaluated using 
    /// floating-point operations, rather than using the methods provided by `rust_decimal` (which
    /// typically use Taylor series expansions). This produces less accurate results, but is much
    /// faster.
    pub use_floats: bool,
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

            StructuredNode::Number(_) | StructuredNode::Sqrt(_) | StructuredNode::Parentheses(_) | StructuredNode::Variable(_) | StructuredNode::Power(_, _) | StructuredNode::FunctionCall(_, _)
                => self.clone(),
        })
    }

    /// Evaluates this node into a single number.
    /// 
    /// Using the [Evaluable](crate::evaluate::Evaluable) trait is more desirable than calling this
    /// method directly, but this still exists for backwards-compatibility.
    pub fn evaluate(&self, settings: &EvaluationSettings) -> Result<Number, MathsError> {
        match self {
            StructuredNode::Number(n) => Ok((*n).into()),
            StructuredNode::Variable(_) => Err(MathsError::MissingVariable),
            StructuredNode::Sqrt(inner) =>
                inner.evaluate(settings)?.to_decimal().sqrt().map(|x| x.into()).ok_or(MathsError::InvalidSqrt),
            StructuredNode::Power(b, e) => b.evaluate(settings)?.checked_pow(e.evaluate(settings)?),
            StructuredNode::Add(a, b) => a.evaluate(settings)?.checked_add(b.evaluate(settings)?),
            StructuredNode::Subtract(a, b) => a.evaluate(settings)?.checked_sub(b.evaluate(settings)?),
            StructuredNode::Multiply(a, b) => a.evaluate(settings)?.checked_mul(b.evaluate(settings)?),
            StructuredNode::Divide(a, b) => a.evaluate(settings)?.checked_div(b.evaluate(settings)?),
            StructuredNode::Parentheses(inner) => inner.evaluate(settings),
            StructuredNode::FunctionCall(func, args) => {
                let args = args.iter().map(|n| n.evaluate(settings)).collect::<Result<Vec<_>, _>>()?;
                func.evaluate(&args, settings)
            }
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
            StructuredNode::FunctionCall(_, args) => {
                for arg in args {
                    arg.walk(func);
                }
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
            StructuredNode::FunctionCall(_, args) => {
                for arg in args {
                    arg.walk_mut(func);
                }
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
fn layout_binop(renderer: &mut impl Renderer, glyph: Glyph, properties: LayoutComputationProperties, left: &StructuredNode, right: &StructuredNode) -> LayoutBlock {
    // These are structured nodes, which (currently) never have a cursor

    let left_layout = left.layout(renderer, None, properties);
    let binop_layout = LayoutBlock::from_glyph(renderer, glyph, properties)
        .move_right_of_other(&left_layout);
    let right_layout = right.layout(renderer, None, properties)
        .move_right_of_other(&binop_layout);

    left_layout
        .merge_along_baseline(&binop_layout)
        .merge_along_baseline(&right_layout)
}

impl Layoutable for StructuredNode {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> LayoutBlock {
        match self {
            StructuredNode::Number(Number::Decimal(mut number, _)) => {
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
                    .map(|g| LayoutBlock::from_glyph(renderer, g, properties))
                    .collect::<Vec<_>>();

                if negative {
                    glyph_layouts.insert(
                        0, 
                        LayoutBlock::from_glyph(renderer, Glyph::Subtract, properties)
                    )
                }

                LayoutBlock::layout_horizontal(&glyph_layouts[..])
            },
            StructuredNode::Number(Number::Rational(numer, denom)) => {
                if *denom == 1 {
                    StructuredNode::Number(Number::Decimal(Decimal::from_i64(*numer).unwrap(), DecimalAccuracy::Exact)).layout(renderer, path, properties)
                } else {
                    common::layout_fraction(
                        &StructuredNode::Number(Number::Decimal(Decimal::from_i64(*numer).unwrap(), DecimalAccuracy::Exact)),
                        &StructuredNode::Number(Number::Decimal(Decimal::from_i64(*denom).unwrap(), DecimalAccuracy::Exact)),
                        renderer,
                        None,
                        properties,
                    )
                }
            },

            StructuredNode::Variable(v) => LayoutBlock::from_glyph(renderer, Glyph::Variable { name: *v }, properties),

            StructuredNode::Add(left, right) => layout_binop(renderer, Glyph::Add, properties, left, right),
            StructuredNode::Subtract(left, right) => layout_binop(renderer, Glyph::Subtract, properties, left, right),
            StructuredNode::Multiply(left, right) => layout_binop(renderer, Glyph::Multiply, properties, left, right),

            StructuredNode::Divide(top, bottom)
                => common::layout_fraction(top.deref(), bottom.deref(), renderer, path, properties),
            StructuredNode::Sqrt(inner)
                => common::layout_sqrt(inner.deref(), renderer, path, properties),
            StructuredNode::Parentheses(inner)
                => common::layout_parentheses(inner.deref(), renderer, path, properties),
            StructuredNode::Power(base, exp)
                => common::layout_power(Some(base.deref()), exp.deref(), renderer, path, properties),
            StructuredNode::FunctionCall(func, args)
                => common::layout_function_call(*func, args, renderer, path, properties),
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

            Self::FunctionCall(func, args) => SimplifiedNode::FunctionCall(
                *func,
                args.iter().map(|n| n.simplify()).collect(),
            ),

            Self::Parentheses(n) => n.simplify(),
        }
    }
}

impl crate::evaluate::Evaluable for StructuredNode {
    type Substituted = Self;
    type Settings = EvaluationSettings;

    fn evaluate(self, settings: &Self::Settings) -> Result<Number, MathsError> {
        StructuredNode::evaluate(&self, settings)
    }

    fn substitute(self, variable: char, value: Number) -> Self::Substituted {
        self.substitute_variable(variable, &StructuredNode::Number(value))
    }
}
