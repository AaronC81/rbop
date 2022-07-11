//! The definition of the unstructured node tree itself.

use core::iter::repeat;

use alloc::{vec, vec::Vec, string::ToString};

use crate::{node::function::Function, Number};

/// An unstructured item, either a node or a node list. Useful for making functions which traverse
/// node trees more generic.
#[derive(Clone)]
pub enum UnstructuredItem<'a> {
    Node(&'a UnstructuredNode),
    List(&'a UnstructuredNodeList),
}

/// A token which may appear in an unstructured node tree. These are simple, character-sized items
/// which are simple to draw, with no further nodes nested inside them.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Token {
    /// An addition symbol.
    Add,

    /// A subtraction symbol.
    Subtract,

    /// A multiplication symbol. (It is possible for implicit multiplications to appear too, if
    /// expressions are adjacent.)
    Multiply,

    /// A division symbol. (If you would like the division to appear as a fraction, you may wish to
    /// use [UnstructuredNode::Fraction] instead.)
    Divide,

    /// A base-10 digit.
    Digit(u8),

    /// A decimal point.
    Point,

    /// A variable, denoted by a particular character.
    Variable(char),
}

impl Token {
    /// Attempts to convert the given character to a `Token`, or returns `None` if this is not
    /// possible.
    /// 
    /// For simplicity, this function currently supports only ASCII characters - Unicode
    /// mathematical operators like `×` are not recognised, but `*` would be.
    /// 
    /// Because any character value could be considered valid, this function will not return a
    /// [Token::Variable].
    pub fn from_char(c: char) -> Option<Token> {
        match c {
            '+' => Some(Token::Add),
            '-' => Some(Token::Subtract),
            '*' => Some(Token::Multiply),
            '/' => Some(Token::Divide),
            '.' => Some(Token::Point),
            _ if c.is_digit(10) => Some(Token::Digit(c.to_digit(10).unwrap() as u8)),
            
            _ => None,
        }
    }
}

/// An unstructured node in the tree. See the
/// [module-level documentation](crate::node::unstructured) for more information.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum UnstructuredNode {
    /// A plain token.
    Token(Token),

    /// A square root, applied to other unstructured nodes.
    Sqrt(UnstructuredNodeList),

    /// A fraction/division, with two other lists of unstructured nodes as the numerator and
    /// denominator.
    Fraction(UnstructuredNodeList, UnstructuredNodeList),

    /// A set of parentheses containing other unstructured nodes.
    Parentheses(UnstructuredNodeList),
    
    /// A power of unstructured nodes. This node does not encode the base of the power - this is
    /// only discovered by upgrading the tree.
    Power(UnstructuredNodeList),

    /// A function call, with a sequence of arguments passed as unstructured nodes. 
    FunctionCall(Function, Vec<UnstructuredNodeList>),
}

impl UnstructuredNode {
    /// Creates a new `UnstructuredNode::FunctionCall` given a function.
    pub fn new_function_call(func: Function) -> Self {
        let arg_vec = repeat(UnstructuredNodeList::new()).take(func.argument_count()).collect();
        Self::FunctionCall(func, arg_vec)
    }
}

/// An ordered sequence of unstructured nodes.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct UnstructuredNodeList {
    pub items: Vec<UnstructuredNode>
}

impl UnstructuredNodeList {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The root of a tree of unstructured nodes.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct UnstructuredNodeRoot {
    pub root: UnstructuredNodeList
}

impl UnstructuredNodeRoot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `UnstructuredNodeRoot` given a number.
    /// 
    /// `Decimal`s and whole `Rational`s become a sequence of tokens. `Rational`s with a denominator
    /// greater than 1 become a `Fraction` with a sequence of tokens on the top and bottom.
    pub fn from_number(num: Number) -> Self {
        fn str_to_nodes(s: &str) -> Vec<UnstructuredNode> {
            s.chars()
                .map(|c| UnstructuredNode::Token(
                    Token::from_char(c).expect("unknown token in decimal")
                ))
                .collect::<Vec<_>>()
        }

        Self {
            root: UnstructuredNodeList {
                items: match num {
                    Number::Decimal(d, _) => str_to_nodes(&d.to_string()),

                    Number::Rational(numer, denom) => {
                        if denom == 1 {
                            str_to_nodes(&numer.to_string())
                        } else {
                            vec![UnstructuredNode::Fraction(
                                UnstructuredNodeList { items: str_to_nodes(&numer.to_string()) },
                                UnstructuredNodeList { items: str_to_nodes(&denom.to_string()) },
                            )]
                        }
                    },
                }
            }
        }
    }
}
