//! The compiled node tree, which is immutable and opaque but evaluates very quickly.
//! 
//! Compiled nodes are created by transforming [structured](crate::node::structured) nodes. All
//! information about the structure and composition of the node tree is discarded, leaving only an
//! object which can be used to evaluate the expression represented by the structured node tree.
//! 
//! Specifically, the compilation process transforms each node into a closure which calls other
//! closures. The top-level closure is held as a trait object in a [Box] by the [CompiledNode]
//! instance, and can be called to evaluate the expression, either with
//! [evaluate_raw](CompiledNode::evaluate_raw) (faster) or with the [Evaluable] trait (slower, but 
//! has a common interface with structured nodes).
//! 
//! Compiled nodes are always parameterised on exactly one variable, the name of which must be
//! specified during compilation. The variable's value is passed as an argument between the
//! closures. If parameterisation is not needed, just pass a dummy value when evaluating the
//! expression.
//! 
//! Once compiled, compiled nodes have been observed to be over twice as fast as evaluating the 
//! structured node tree that they were compiled from. Combined with their single parameter, this 
//! makes them perfect for the repeated evaluations required for plotting a graph.

use alloc::{boxed::Box, vec::Vec};
use num_traits::Zero;
use rust_decimal::MathematicalOps;

use crate::{Number, StructuredNode, error::MathsError, evaluate::Evaluable};

use super::structured::EvaluationSettings;

/// A compiled node. See the [module-level documentation](crate::node::compiled) for more
/// information.
pub struct CompiledNode {
    /// The top-level closure which can be called to evaluate this expression. This is called with
    /// the evaluation parameter value.
    func: Box<dyn Fn(&Number) -> Result<Number, MathsError>>,

    /// The name of the parameter variable (if any) which this was compiled with. Not necessary for
    /// evaluation - kept only for validation within [Evaluable::substitute].
    param_var: Option<char>,
}

impl CompiledNode {
    /// Create a new compiled node wrapping the given function.
    pub fn new(func: impl Fn(&Number) -> Result<Number, MathsError> + 'static, param_var: Option<char>) -> Self {
        Self { func: Box::new(func), param_var }
    }

    /// Compiles a [StructuredNode] tree, optionally parameterised on the given `param_var`.
    pub fn from_structured(node: StructuredNode, param_var: Option<char>, evaluation_settings: &EvaluationSettings) -> Self {
        match node {
            StructuredNode::Number(n) => Self::new(move |_| Ok(n), param_var),
            StructuredNode::Variable(name) => {
                if Some(name) == param_var {
                    Self::new(|n| Ok(*n), param_var)
                } else {
                    Self::new(|_| Err(MathsError::MissingVariable), param_var)
                }
            },
            StructuredNode::Sqrt(inner) => {
                let inner = Self::from_structured(*inner, param_var, evaluation_settings);
                Self::new(
                    move |n| Ok((inner.func)(n)?.to_decimal().sqrt().ok_or(MathsError::InvalidSqrt)?.into()),
                    param_var
                )
            }
            StructuredNode::Power(base, exp) => {
                let base = Self::from_structured(*base, param_var, evaluation_settings);
                let exp = Self::from_structured(*exp, param_var, evaluation_settings);
                Self::new(move |n| (base.func)(n)?.checked_pow((exp.func)(n)?), param_var)
            },
            StructuredNode::Add(left, right) => {
                let left = Self::from_structured(*left, param_var, evaluation_settings);
                let right = Self::from_structured(*right, param_var, evaluation_settings);
                Self::new(move |n| (left.func)(n)?.checked_add((right.func)(n)?), param_var)
            }
            StructuredNode::Subtract(left, right) => {
                let left = Self::from_structured(*left, param_var, evaluation_settings);
                let right = Self::from_structured(*right, param_var, evaluation_settings);
                Self::new(move |n| (left.func)(n)?.checked_sub((right.func)(n)?), param_var)
            }
            StructuredNode::Multiply(left, right) => {
                let left = Self::from_structured(*left, param_var, evaluation_settings);
                let right = Self::from_structured(*right, param_var, evaluation_settings);
                Self::new(move |n| (left.func)(n)?.checked_mul((right.func)(n)?), param_var)
            }
            StructuredNode::Divide(left, right) => {
                let left = Self::from_structured(*left, param_var, evaluation_settings);
                let right = Self::from_structured(*right, param_var, evaluation_settings);
                Self::new(move |n| (left.func)(n)?.checked_div((right.func)(n)?), param_var)
            }
            StructuredNode::Parentheses(inner) => Self::from_structured(*inner, param_var, evaluation_settings),
            StructuredNode::FunctionCall(func, args) => {
                let arg_funcs = args.into_iter().map(|arg| Self::from_structured(arg, param_var, evaluation_settings)).collect::<Vec<_>>();
                let settings_clone = evaluation_settings.clone();
                Self::new(move |n| {
                    let evaluated_args = arg_funcs.iter().map(|cn| (cn.func)(n)).collect::<Result<Vec<_>, _>>()?;
                    func.evaluate(&evaluated_args, &settings_clone)
                }, param_var)
            }
        }
    }

    /// Evaluates a compiled node with the given parameter.
    /// 
    /// If this was compiled with a parameter variable, the given parameter will substitute
    /// occurences of that variable. Otherwise, you can just pass a dummy value.
    pub fn evaluate_raw(&self, param: Number) -> Result<Number, MathsError> {
        (self.func)(&param)
    }
}

/// A named constant for the settings to pass as the settings for a [CompiledNode] to
/// [Evaluable::evaluate].
pub const COMPILED_NODE_SETTINGS: &'static () = &();

impl Evaluable for CompiledNode {
    type Substituted = CompiledNodeEvaluableSubstituted;
    type Settings = ();

    fn evaluate(self, _settings: &Self::Settings) -> Result<Number, MathsError> {
        if self.param_var.is_some() {
            return Err(MathsError::MissingVariable);
        }

        (self.func)(&Number::zero())
    }

    fn substitute(self, variable: char, value: Number) -> Self::Substituted {
        if self.param_var != Some(variable) {
            panic!("cannot substitute {} in node which was compiled for {:?}", variable, self.param_var);
        }

        CompiledNodeEvaluableSubstituted {
            node: self,
            value,
        }
    }
}

/// An intermediary type holding the parameter value when calling [Evaluable::substitute] on a
/// [CompiledNode].
/// 
/// This also implements [Evaluable], but **substituting multiple times is not allowed**. Calling
/// [Evaluable::substitute] on this type will panic.
pub struct CompiledNodeEvaluableSubstituted {
    node: CompiledNode,
    value: Number,
}

impl Evaluable for CompiledNodeEvaluableSubstituted {
    type Substituted = CompiledNodeEvaluableSubstituted; // ...but not really
    type Settings = ();

    fn evaluate(self, _settings: &Self::Settings) -> Result<Number, MathsError> {
        (self.node.func)(&self.value)
    }

    fn substitute(self, _variable: char, _value: Number) -> Self::Substituted {
        panic!("cannot substitute compiled node variable more than once")
    }
}
