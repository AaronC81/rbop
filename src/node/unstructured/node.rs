use core::iter::repeat;

use alloc::vec::Vec;

use crate::node::function::Function;


#[derive(Clone)]
pub enum UnstructuredItem<'a> {
    Node(&'a UnstructuredNode),
    List(&'a UnstructuredNodeList),
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Token {
    Add,
    Subtract,
    Multiply,
    Divide,
    Digit(u8),
    Point,
    Variable(char),
}

/// An unstructured node is one which can be inputted by the user. Unstructured nodes have as little
/// structure as possible - for example, "2+3*5" is represented as a flat list of tokens, with no
/// respect for precedence.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum UnstructuredNode {
    Token(Token),
    Sqrt(UnstructuredNodeList),
    Fraction(UnstructuredNodeList, UnstructuredNodeList),
    Parentheses(UnstructuredNodeList),
    Power(UnstructuredNodeList),
    FunctionCall(Function, Vec<UnstructuredNodeList>),
}

impl UnstructuredNode {
    /// Creates a new `UnstructuredNode::FunctionCall` given a function.
    pub fn new_function_call(func: Function) -> Self {
        let arg_vec = repeat(UnstructuredNodeList::new()).take(func.argument_count()).collect();
        Self::FunctionCall(func, arg_vec)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct UnstructuredNodeList {
    pub items: Vec<UnstructuredNode>
}

impl UnstructuredNodeList {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct UnstructuredNodeRoot {
    pub root: UnstructuredNodeList
}

impl UnstructuredNodeRoot {
    pub fn new() -> Self {
        Self::default()
    }
}
