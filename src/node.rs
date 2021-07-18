use std::fmt;
use std::error::Error;

use crate::nav::{NavPath, NavPathNavigator};

pub type Number = i128;

pub struct Parser<'a> {
    nodes: &'a [Node],
    index: usize,
}

#[derive(Debug, Clone)]
pub struct NodeError(String);

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for NodeError {}

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
            Err(box NodeError("expected a number".into()))
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Token {
    Add,
    Subtract,
    Multiply,
    Divide,
    Digit(u8),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Node {
    Number(Number),
    Token(Token),
    Sqrt(Box<Node>),
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Parentheses(Box<Node>),
    Unstructured(Vec<Node>),
}

impl Node {
    /// Returns true if this node is `Add` or `Subtract`.
    pub fn add_or_sub(&self) -> bool {
        matches!(&self, Node::Add(_, _) | Node::Subtract(_, _))
    }

    /// Returns true if this node is `Multiply` or `Divide`.
    pub fn mul_or_div(&self) -> bool {
        matches!(&self, Node::Multiply(_, _) | Node::Divide(_, _))
    }

    /// Returns a clone of this node wrapped in `Parentheses`.
    pub fn in_parentheses(&self) -> Node {
        Node::Parentheses(box self.clone())
    }

    /// If `parens` is true, returns a clone of this node wrapped in `Parentheses`, otherwise just
    /// returns a plain clone of this node.
    pub fn in_parentheses_or_clone(&self, parens: bool) -> Node {
        if parens {
            self.in_parentheses()
        } else {
            self.clone()
        }
    }
    
    /// Returns a clone of this node tree where all unstructured nodes have exactly one child, and
    /// that child is not a `Token`.
    pub fn upgrade(&self) -> Result<Node, Box<dyn Error>> {
        Ok(match self {
            // These are all simple tree walks
            Node::Add(l, r) => Node::Add(box l.upgrade()?, box r.upgrade()?),
            Node::Subtract(l, r) => Node::Subtract(box l.upgrade()?, box r.upgrade()?),
            Node::Multiply(l, r) => Node::Multiply(box l.upgrade()?, box r.upgrade()?),
            Node::Divide(l, r) => Node::Divide(box l.upgrade()?, box r.upgrade()?),
            Node::Sqrt(n) => Node::Sqrt(box n.upgrade()?),
            Node::Number(_) | Node::Token(_) => self.clone(),
            Node::Parentheses(n) => Node::Parentheses(box n.upgrade()?),

            // Upgrading an unstructured node involves parsing it
            Node::Unstructured(nodes) => Parser {
                index: 0,
                nodes: &nodes[..]
            }.parse()?
        })
    }

    /// Returns a clone of this node tree with added parentheses to show the order of operations
    /// when the tree is rendered.
    /// The tree should be upgraded before doing this.
    pub fn disambiguate(&self) -> Result<Node, Box<dyn Error>> {
        Ok(match self {
            // We need to add parentheses around:
            //   - operations which mix precedence, e.g. (3+2)*4
            //   - operations which go against standard associativity for - and /, e.g. 3-(3-2)

            Node::Multiply(l, r) => {
                let l = l.in_parentheses_or_clone(l.add_or_sub());
                let r = r.in_parentheses_or_clone(r.add_or_sub() || r.mul_or_div());
                Node::Multiply(box l, box r)
            }
            Node::Divide(l, r) => {
                let l = l.in_parentheses_or_clone(l.add_or_sub());
                let r = r.in_parentheses_or_clone(r.add_or_sub() || r.mul_or_div());
                Node::Divide(box l, box r)
            }

            Node::Add(l, r) => {
                let r = r.in_parentheses_or_clone(r.add_or_sub());
                Node::Add(l.clone(), box r)
            }
            Node::Subtract(l, r) => {
                let r = r.in_parentheses_or_clone(r.add_or_sub());
                Node::Subtract(l.clone(), box r)
            }

            Node::Number(_) | Node::Sqrt(_) | Node::Parentheses(_) => self.clone(),

            Node::Unstructured(_) | Node::Token(_) => return Err(box NodeError(
                "attempting to disambiguate non-upgraded tree".into()
            ))
        })
    }

    /// Given a navigation path, returns the node from following that path, and
    /// the index into that node.
    /// The navigation path will always terminate on an unstructured node, so
    /// the final index in the path will be an index into the unstructured
    /// node's items.
    pub fn navigate_mut(&mut self, path: &mut NavPathNavigator) -> (&mut Node, usize) {
        if path.here() {
            if !matches!(self, &mut Node::Unstructured(_)) {
                panic!("navigation path must end on unstructured node");
            }
            return (self, path.next())
        }

        let next_index = path.next();
        let step_path = &mut path.step();

        match self {
            Node::Sqrt(inner) => {
                if next_index != 0 {
                    panic!("index out of range for sqrt navigation")
                }

                inner.navigate_mut(step_path)
            },
            Node::Unstructured(items) => {
                items[next_index].navigate_mut(step_path)
            },
            Node::Divide(top, bottom) => {
                if next_index == 0 {
                    top.navigate_mut(step_path)
                } else if next_index == 1 {
                    bottom.navigate_mut(step_path)
                } else {
                    panic!("index out of range for divide navigation")
                }
            },
            
            Node::Number(_) | Node::Token(_) => panic!("cannot navigate into this"),

            Node::Add(_, _)
            | Node::Subtract(_, _)
            | Node::Multiply(_, _)
            | Node::Parentheses(_) => panic!("cannot navigate into structured node"),
        }
    }
}
