use alloc::{boxed::Box, vec};
use rust_decimal::{Decimal, MathematicalOps, prelude::ToPrimitive};

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

        let mut result = if let Some(Token::Digit(d)) = self.current_token() {
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

            // Is the next token a decimal point?
            if let Some(Token::Point) = self.current_token() {
                self.advance();

                // Alright, this could have a decimal part - is there a digit after the point?
                // (If not, that's fine, do nothing - we accept "3.")
                if let Some(Token::Digit(_)) = self.current_token() {
                    // Yes - recurse, without advancing (since we want this parser function to pick
                    // up that first digit)
                    let decimal_part = self.parse_level3()?;

                    // The parse result must be an integer, otherwise the input may have been
                    // something like "12.34.56" which is invalid
                    if let StructuredNode::Number(dec_part) = decimal_part {
                        if dec_part.fract() != Decimal::ZERO {
                            return Err(box NodeError("multiple decimal points".into()))
                        }
                        if dec_part.is_sign_negative() {
                            return Err(box NodeError("decimal part must be positive".into()))
                        }

                        if dec_part != Decimal::ZERO {
                            // Not sure what the best way to do this is - probably not this, but it
                            // does work!
                            // Example, for "123.45"

                            // 1. Get the "length" of the decimal part, e.g. "45" has length 2
                            let length_of_decimal_part = dec_part.log10().floor() + Decimal::ONE;
                            // 2. Multiply whole part by 10^length, = "12300."
                            number *= Decimal::TEN.powd(length_of_decimal_part);
                            // 3. Add decimal part, = "12345."
                            number += dec_part;
                            // 4. Shift point by length of decimal part, = "123.45"
                            number.set_scale(number.scale() + length_of_decimal_part.to_u32().unwrap()).unwrap();
                        }
                    } else {
                        return Err(box NodeError("expected number after decimal point".into()))
                    }
                }
            }

            StructuredNode::Number(number)
        } else if let Some(UnstructuredNode::Fraction(a, b)) = self.current() {
            self.advance();
            StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?)
        } else if let Some(UnstructuredNode::Sqrt(n)) = self.current() {
            self.advance();
            StructuredNode::Sqrt(box n.upgrade()?)
        } else if let Some(UnstructuredNode::Parentheses(inner)) = self.current() {
            self.advance();
            StructuredNode::Parentheses(box inner.upgrade()?)
        } else if let Some(Token::Variable(v)) = self.current_token() {
            self.advance();
            StructuredNode::Variable(v)
        } else {
            return Err(box NodeError("expected a unit".into()))
        };

        // Construct implicit multiplications as long as the next token is one which can be
        // implicitly multipled with. "2x" will initially parse as "2", then this pass can pick up
        // the "x" and form a multiplication.
        while matches!(
            self.current(),
            Some(
                UnstructuredNode::Fraction(_, _)
                | UnstructuredNode::Sqrt(_)
                | UnstructuredNode::Parentheses(_)
                | UnstructuredNode::Token(Token::Variable(_) | Token::Digit(_))
            )
        ) {
            result = StructuredNode::Multiply(box result, box self.parse_level3()?);
        }

        Ok(result)
    }
}
