use core::iter::repeat;

use alloc::{vec, vec::Vec, string::ToString};

use crate::{node::function::Function, Number};


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
