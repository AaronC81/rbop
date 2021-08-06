use alloc::boxed::Box;
use rust_decimal::Decimal;

use crate::error::{Error, NodeError};

use super::{structured::StructuredNode, unstructured::{Token, UnstructuredNode, Upgradable}};

/// Converts a list of unstructured nodes into a single structured node. Used to implement
/// `Upgradable` for `UnstructuredNodeList`.
pub struct Parser<'a> {
    pub nodes: &'a [UnstructuredNode],
    pub index: usize,
}

impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> Result<StructuredNode, Box<dyn Error>> {
        self.parse_level1()
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn current(&mut self) -> Option<&'a UnstructuredNode> {
        if self.index < self.nodes.len() {
            Some(&self.nodes[self.index])
        } else {
            None
        }
    }

    fn current_token(&mut self) -> Option<Token> {
        if let Some(UnstructuredNode::Token(t)) = self.current() {
            Some(*t)
        } else {
            None
        }
    }

    fn eoi(&mut self) -> bool {
        self.index >= self.nodes.len()
    }

    fn parse_level1(&mut self) -> Result<StructuredNode, Box<dyn Error>> {
        let mut out = self.parse_level2()?;

        while !self.eoi() {
            if let Some(op @ (Token::Add | Token::Subtract)) = self.current_token() {
                self.advance();

                let left = out.clone();
                if op == Token::Add {
                    out = StructuredNode::Add(box left, box self.parse_level2()?);
                } else if op == Token::Subtract {
                    out = StructuredNode::Subtract(box left, box self.parse_level2()?);
                } else {
                    unreachable!()
                }
            } else {
                break;
            }
        }

        Ok(out)
    }

    fn parse_level2(&mut self) -> Result<StructuredNode, Box<dyn Error>> {
        let mut out = self.parse_level3()?;

        while !self.eoi() {
            if let Some(op @ (Token::Multiply | Token::Divide)) = self.current_token() {
                self.advance();

                let left = out.clone();
                if op == Token::Multiply {
                    out = StructuredNode::Multiply(box left, box self.parse_level3()?);
                } else if op == Token::Divide {
                    out = StructuredNode::Divide(box left, box self.parse_level3()?);
                } else {
                    unreachable!()
                }
            } else {
                break;
            }
        }

        Ok(out)
    }

    fn parse_level3(&mut self) -> Result<StructuredNode, Box<dyn Error>> {
        // while loop and flipping allows multiple unary minuses
        let mut parsed_number_is_negative = false;
        while let Some(Token::Subtract) = self.current_token() {
            self.advance();
            parsed_number_is_negative = !parsed_number_is_negative;
        }

        if let Some(Token::Digit(d)) = self.current_token() {
            // Parse a number made of digits
            let mut number: Decimal = d.into();
            self.advance();

            while !self.eoi() {
                if let Some(Token::Digit(d)) = self.current_token() {
                    number *= Decimal::from(10);
                    number += Decimal::from(d);

                    self.advance();
                } else {
                    break;
                }
            }

            if parsed_number_is_negative {
                number = -number;
            }

            Ok(StructuredNode::Number(number))
        } else if let Some(UnstructuredNode::Fraction(a, b)) = self.current() {
            // Divisions can appear in unstructured nodes - upgrade the children
            self.advance();
            Ok(StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?))
        } else if let Some(UnstructuredNode::Sqrt(n)) = self.current() {
            // Sqrt can appear in unstructured nodes - upgrade the child
            self.advance();
            Ok(StructuredNode::Sqrt(box n.upgrade()?))
        } else {
            Err(box NodeError("expected a unit".into()))
        }
    }
}
