use alloc::{string::ToString, vec::Vec};
use num_traits::Zero;
use rust_decimal::{Decimal, prelude::{FromPrimitive, ToPrimitive}, MathematicalOps};

use crate::{Number, error::NodeError, number::DecimalAccuracy};

use super::{structured::StructuredNode, unstructured::{Token, UnstructuredNode, Upgradable}};

/// Converts a list of unstructured nodes into a single structured node. Used to implement
/// `Upgradable` for `UnstructuredNodeList`.
pub struct Parser<'a> {
    pub nodes: &'a [UnstructuredNode],
    pub index: usize,
}

impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> Result<StructuredNode, NodeError> {
        let result = self.parse_level1()?;

        // Leftover tokens is an error
        if self.index < self.nodes.len() {
            Err(NodeError::UnexpectedTokensAtEnd)
        } else {
            Ok(result)
        }
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

    /// Designed to wrap the return value of a `parse_xxx` function, indicating that this node type
    /// can have a power bound to it. If a Power is the next unstructured node, returns a Power
    /// structured node. Otherwise returns the original node parameter.
    ///
    /// Returns a Result since the exponent node will need to be upgraded if a Power is found.
    fn accepts_power(&mut self, node: StructuredNode) -> Result<StructuredNode, NodeError> {
        let mut result = node;
        while let Some(UnstructuredNode::Power(exp)) = self.current() {
            self.advance();
            result = StructuredNode::Power(
                box result,
                box exp.upgrade()?,
            )
        }

        Ok(result)
    } 

    fn parse_level1(&mut self) -> Result<StructuredNode, NodeError> {
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

    fn parse_level2(&mut self) -> Result<StructuredNode, NodeError> {
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

    fn parse_level3(&mut self) -> Result<StructuredNode, NodeError> {
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
                    number = number.checked_mul(Decimal::from(10u8)).ok_or(NodeError::Overflow)?;
                    number = number.checked_add(Decimal::from(d)).ok_or(NodeError::Overflow)?;

                    self.advance();
                } else {
                    break;
                }
            }

            let mut is_decimal = false;

            // Is the next token a decimal point?
            if let Some(Token::Point) = self.current_token() {
                self.advance();

                // 3. should be parsed as a decimal, not a rational
                is_decimal = true;

                // Alright, this could have a decimal part - is there a digit after the point?
                // (If not, that's fine, do nothing - we accept "3.")
                if let Some(Token::Digit(_)) = self.current_token() {
                    // Yes - collect decimal part
                    // We have to keep the number of leading 0s separately too, since 0 * 10 + 0 is
                    // a no-op
                    let mut dec_part = Decimal::zero();
                    let mut collect_leading_zeros = true;
                    let mut leading_zeros_count = 0;
                    while !self.eoi() {
                        if let Some(Token::Digit(d)) = self.current_token() {
                            if collect_leading_zeros && d == 0 {
                                leading_zeros_count += 1;
                            } else {
                                collect_leading_zeros = false;
                                dec_part *= Decimal::from(10u8);
                                dec_part += Decimal::from(d);
                            }
        
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    if dec_part != Decimal::zero() {
                        // Not sure what the best way to do this is - probably not this, but it
                        // does work!
                        // Example, for "123.45"

                        // 1. Get the "length" of the decimal part, e.g. "45" has length 2
                        let length_of_decimal_part =
                            // TODO: implement proper log10
                            dec_part.to_string().len()
                            + leading_zeros_count;
                        // 2. Multiply whole part by 10^length, = "12300."
                        number *= Decimal::from_u8(10).unwrap().powi(length_of_decimal_part as i64);
                        // 3. Add decimal part, = "12345."
                        number += dec_part;
                        // 4. Shift point by length of decimal part, = "123.45"
                        number.set_scale(number.scale() + length_of_decimal_part.to_u32().unwrap()).unwrap();
                    }
                }
            }

            self.accepts_power(StructuredNode::Number(
                if is_decimal {
                    Number::Decimal(number, DecimalAccuracy::Exact)
                } else {
                    // Handle case where number doesn't fit in i64
                    if let Some(numerator) = number.to_i64() {
                        Number::Rational(numerator, 1)
                    } else {
                        Number::Decimal(number, DecimalAccuracy::Exact)
                    }
                }
            ))?
        } else if let Some(UnstructuredNode::Fraction(a, b)) = self.current() {
            self.advance();
            self.accepts_power(StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?))?
        } else if let Some(UnstructuredNode::Sqrt(n)) = self.current() {
            self.advance();
            self.accepts_power(StructuredNode::Sqrt(box n.upgrade()?))?
        } else if let Some(UnstructuredNode::Parentheses(inner)) = self.current() {
            self.advance();
            self.accepts_power(StructuredNode::Parentheses(box inner.upgrade()?))?
        } else if let Some(UnstructuredNode::Power(_)) = self.current() {
            return Err(NodeError::PowerMissingBase)
        } else if let Some(Token::Variable(v)) = self.current_token() {
            self.advance();
            self.accepts_power(StructuredNode::Variable(v))?
        } else if let Some(UnstructuredNode::FunctionCall(func, args)) = self.current() {
            self.advance();
            self.accepts_power(StructuredNode::FunctionCall(*func, args.iter().map(|n| n.upgrade()).collect::<Result<Vec<_>, _>>()?))?
        } else {
            return Err(NodeError::ExpectedUnit)
        };

        if parsed_number_is_negative {
            if let StructuredNode::Number(number) = &mut result {
                *number *= Number::Rational(-1, 1);
            } else {
                result = StructuredNode::Multiply(box StructuredNode::Number(Number::Rational(-1, 1)), box result);
            }
        }

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
