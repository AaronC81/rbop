use alloc::{vec::Vec, vec};

use crate::{serialize::Serializable, UnstructuredNodeRoot, UnstructuredNodeList, UnstructuredNode, Token, node::function::Function};

impl Serializable for UnstructuredNodeRoot {
    fn serialize(&self) -> Vec<u8> {
        self.root.serialize()
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        Some(UnstructuredNodeRoot {
            root: UnstructuredNodeList::deserialize(bytes)?
        })
    }
}

impl Serializable for UnstructuredNode {
    fn serialize(&self) -> Vec<u8> {
        match self {
            UnstructuredNode::Token(t) => {
                let mut token_bytes = t.serialize();
                if token_bytes[0] > 0b01111111 { panic!(); }

                token_bytes[0] |= 0b10000000;
                token_bytes
            },
            UnstructuredNode::Sqrt(i) => {
                let mut n = vec![1];
                n.append(&mut i.serialize());
                n
            },
            UnstructuredNode::Fraction(t, b) => {
                let mut n = vec![2];
                n.append(&mut t.serialize());
                n.append(&mut b.serialize());
                n
            }
            UnstructuredNode::Parentheses(i) => {
                let mut n = vec![3];
                n.append(&mut i.serialize());
                n
            },
            UnstructuredNode::Power(e) => {
                let mut n = vec![4];
                n.append(&mut e.serialize());
                n
            },
            UnstructuredNode::FunctionCall(func, args) => {
                let mut n = vec![5];
                n.append(&mut func.serialize());
                n.append(&mut vec![args.len() as u8]);
                for arg in args {
                    n.append(&mut arg.serialize());
                }
                n
            }
        }
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        let first_byte = bytes.next()?;
        match first_byte {
            _ if first_byte & 0b10000000 > 0 =>
                Some(UnstructuredNode::Token(
                    Token::deserialize(&mut vec![first_byte & 0b01111111]
                        .into_iter()
                        .chain(bytes))?)
                ),
            1 => Some(UnstructuredNode::Sqrt(UnstructuredNodeList::deserialize(bytes)?)),
            2 => Some(UnstructuredNode::Fraction(
                UnstructuredNodeList::deserialize(bytes)?,
                UnstructuredNodeList::deserialize(bytes)?,
            )),
            3 => Some(UnstructuredNode::Parentheses(UnstructuredNodeList::deserialize(bytes)?)),
            4 => Some(UnstructuredNode::Power(
                UnstructuredNodeList::deserialize(bytes)?,
            )),
            5 => {
                let func = Function::deserialize(bytes)?;
                let arg_count = bytes.next()?;
                let mut args = vec![];
                for _ in 0..arg_count {
                    args.push(UnstructuredNodeList::deserialize(bytes)?);
                }
                Some(UnstructuredNode::FunctionCall(func, args))
            },

            _ => None,
        }
    }
}

impl Serializable for UnstructuredNodeList {
    fn serialize(&self) -> Vec<u8> {
        let mut result = vec![];
        result.append(&mut self.items.len().serialize());
        for item in &self.items {
            result.append(&mut item.serialize());
        }
        result
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        let len = usize::deserialize(bytes)?;
        let mut result = vec![];
        for _ in 0..len {
            result.push(UnstructuredNode::deserialize(bytes)?);
        }
        Some(UnstructuredNodeList { items: result })
    }
}

impl Serializable for Token {
    fn serialize(&self) -> Vec<u8> {
        vec![match self {
            Token::Add => 1,
            Token::Subtract => 2,
            Token::Multiply => 3,
            Token::Divide => 4,
            Token::Digit(d) => 5 + *d,
            Token::Point => 15,
            Token::Variable(c) => return vec![16, *c as u8],
        }]
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        let byte = bytes.next()?;
        Some(match byte {
            1 => Token::Add,
            2 => Token::Subtract,
            3 => Token::Multiply,
            4 => Token::Divide,
            5..=14 => Token::Digit(byte - 5),
            15 => Token::Point,
            16 => Token::Variable(bytes.next()? as char),

            _ => return None,
        })
    }
}
