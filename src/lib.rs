#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(or_patterns)]

use std::fmt;
use std::error::Error;

type Number = i128;

struct Parser<'a> {
    nodes: &'a [Node],
    index: usize,
}

#[derive(Debug, Clone)]
struct ParserError(String);

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for ParserError {}

impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> Result<Node, Box<dyn Error>> {
        self.parse_level1()
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn current(&mut self) -> &'a Node {
        &self.nodes[self.index]
    }

    fn current_token(&mut self) -> Option<Token> {
        if let Node::Token(t) = self.current() {
            Some(*t)
        } else {
            None
        }
    }

    fn eoi(&mut self) -> bool {
        self.index >= self.nodes.len()
    }

    fn parse_level1(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut out = self.parse_level2()?;

        while !self.eoi() {
            if let Some(op @ (Token::Add | Token::Subtract)) = self.current_token() {
                self.advance();

                let left = out.clone();
                if op == Token::Add {
                    out = Node::Add(box left, box self.parse_level2()?);
                } else if op == Token::Subtract {
                    out = Node::Subtract(box left, box self.parse_level2()?);
                } else {
                    unreachable!()
                }
            } else {
                break;
            }
        }

        Ok(out)
    }

    fn parse_level2(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut out = self.parse_level3()?;

        while !self.eoi() {
            if let Some(op @ (Token::Multiply | Token::Divide)) = self.current_token() {
                self.advance();

                let left = out.clone();
                if op == Token::Multiply {
                    out = Node::Multiply(box left, box self.parse_level3()?);
                } else if op == Token::Divide {
                    out = Node::Divide(box left, box self.parse_level3()?);
                } else {
                    unreachable!()
                }
            } else {
                break;
            }
        }

        Ok(out)
    }

    fn parse_level3(&mut self) -> Result<Node, Box<dyn Error>> {
        if let Some(Token::Digit(d)) = self.current_token() {
            // Parse a number made of digits
            let mut number = d as Number;
            self.advance();

            while !self.eoi() {
                if let Some(Token::Digit(d)) = self.current_token() {
                    number *= 10;
                    number += d as Number;

                    self.advance();
                } else {
                    break;
                }
            }

            Ok(Node::Number(number))
        } else if let &Node::Number(n) = self.current() {
            // This is already a number, brilliant!
            Ok(self.current().clone())
        } else {
            Err(box ParserError("expected a number".into()))
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum Token {
    Add,
    Subtract,
    Multiply,
    Divide,
    Digit(u8),
}

#[derive(PartialEq, Eq, Debug, Clone)]
enum Node {
    Number(Number),
    Token(Token),
    Sqrt(Box<Node>),
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Unstructured(Vec<Node>),
}

impl Node {
    fn upgrade(&self) -> Result<Node, Box<dyn Error>> {
        Ok(match self {
            // These are all simple tree walks
            Node::Add(l, r) => Node::Add(box l.upgrade()?, box r.upgrade()?),
            Node::Subtract(l, r) => Node::Subtract(box l.upgrade()?, box r.upgrade()?),
            Node::Multiply(l, r) => Node::Multiply(box l.upgrade()?, box r.upgrade()?),
            Node::Divide(l, r) => Node::Divide(box l.upgrade()?, box r.upgrade()?),
            Node::Sqrt(n) => Node::Sqrt(box n.upgrade()?),
            Node::Number(_) | Node::Token(_) => self.clone(),

            // Upgrading an unstructured node involves parsing it
            Node::Unstructured(nodes) => Parser {
                index: 0,
                nodes: &nodes[..]
            }.parse()?
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Node, Token};

    #[test]
    fn upgrade() {
        let unstructured = Node::Unstructured(vec![
            Node::Token(Token::Digit(1)),
            Node::Token(Token::Digit(2)),
            Node::Token(Token::Multiply),
            Node::Token(Token::Digit(3)),
            Node::Token(Token::Digit(4)),
            Node::Token(Token::Add),
            Node::Token(Token::Digit(5)),
            Node::Token(Token::Digit(6)),
            Node::Token(Token::Multiply),
            Node::Token(Token::Digit(7)),
            Node::Token(Token::Digit(8)),
        ]);

        assert_eq!(
            unstructured.upgrade().unwrap(),
            Node::Add(
                box Node::Multiply(
                    box Node::Number(12),
                    box Node::Number(34),
                ),
                box Node::Multiply(
                    box Node::Number(56),
                    box Node::Number(78),
                ),
            )
        );
    }
}

