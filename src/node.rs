use core::fmt;
use alloc::{boxed::Box, string::String, vec::Vec, vec};
use rust_decimal::{Decimal, MathematicalOps};

use crate::{Error, nav::{NavPath, NavPathNavigator}, render::Renderer};

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

#[derive(Debug, Clone)]
pub struct MathsError(String);

impl fmt::Display for MathsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for MathsError {}

// TODO: 1 + (1/2) + 3 is not fully consumed, parsing stops after the fraction. Same with sqrt
impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> Result<Node, Box<dyn Error>> {
        self.parse_level1()
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn current(&mut self) -> Option<&'a Node> {
        if self.index < self.nodes.len() {
            Some(&self.nodes[self.index])
        } else {
            None
        }
    }

    fn current_token(&mut self) -> Option<Token> {
        if let Some(Node::Token(t)) = self.current() {
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
        } else if let Some(Node::Number(_)) = self.current() {
            // This is already a number, brilliant!
            Ok(self.current().unwrap().clone())
        } else if let Some(Node::Divide(a, b)) = self.current() {
            // Divisions can appear in unstructured nodes - upgrade the children
            Ok(Node::Divide(box a.upgrade()?, box b.upgrade()?))
        } else if let Some(Node::Sqrt(n)) = self.current() {
            // Sqrt can appear in unstructured nodes - upgrade the child
            Ok(Node::Sqrt(box n.upgrade()?))
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

pub enum MoveVerticalDirection {
    Up,
    Down,
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
    pub fn navigate(&mut self, path: &mut NavPathNavigator) -> (&mut Node, usize) {
        self.navigate_trace(path, |_| {})
    }

    pub fn navigate_trace<F>(&mut self, path: &mut NavPathNavigator, mut trace: F) -> (&mut Node, usize) 
        where F : FnMut(&mut Node)
    {
        trace(self);

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

                inner.navigate_trace(step_path, trace)
            },
            Node::Unstructured(items) => {
                items[next_index].navigate_trace(step_path, trace)
            },
            Node::Divide(top, bottom) => {
                if next_index == 0 {
                    top.navigate_trace(step_path, trace)
                } else if next_index == 1 {
                    bottom.navigate_trace(step_path, trace)
                } else {
                    panic!("index out of range for divide navigation")
                }
            },
            
            Node::Number(_) | Node::Token(_) => panic!("cannot navigate into this"),
            _ => panic!("cannot navigate into structured node"),
        }
    }

    /// Modifies the given navigation path to move the cursor right.
    pub fn move_right(&mut self, path: &mut NavPath) {
        // Fetch the node which we're navigating within
        let (current_node, index) = self.navigate(&mut path.to_navigator());
        let children = current_node.unwrap_unstructured_mut();

        // Are we at the end of this node?
        if index == children.len() {
            // Is there another node above this one?
            if !path.root() {
                // Move out of the unstructured and the structural node above it
                path.pop(2);

                // Advance past the node which we were inside
                path.offset(1);
            } else {
                // There's nowhere to go, just don't move
            }
        } else {
            // What's to our right?
            let right_child = &children[index];

            match right_child {
                // Structured nodes
                Node::Sqrt(_) | Node::Divide(_, _) => {
                    // Navigate into its first/only slot, and start at the first item of the
                    // unstructured
                    path.push(0);
                    path.push(0);
                },

                // Anything else, we can just move past it
                _ => path.offset(1),
            }
        }
    }

    /// Modifies the given navigation path to move the cursor left.
    pub fn move_left(&mut self, path: &mut NavPath) {
        // Fetch the node which we're navigating within
        let (current_node, index) = self.navigate(&mut path.to_navigator());
        let children = current_node.unwrap_unstructured_mut();

        // Are we at the start of this node?
        if index == 0 {
            // Is there another node above this one?
            if !path.root() {
                // Move out of the unstructured and the structural node above it
                path.pop(2);

                // The index is "before" the node, so no need to offset
            } else {
                // There's nowhere to go, just don't move
            }
        } else {
            // Move left - what's there?
            path.offset(-1);
            let left_child = &children[index - 1];

            match left_child {
                // Structured nodes
                Node::Sqrt(n) | Node::Divide(n, _) => {
                    // Navigate into its first/only slot, and start at the first item of the
                    // unstructured
                    path.push(0);
                    path.push(n.as_ref().unwrap_unstructured().len());
                },

                // Anything else, nothing special needed
                _ => (),
            }
        }
    }

    fn move_vertically(&mut self, path: &mut NavPath, direction: MoveVerticalDirection, renderer: &mut impl Renderer) {
        // Say you're in a sqrt at the top of a fraction, and you press down, you'd expect it to
        // move to the bottom of the fraction.
        // That's why we need to check up the entire nav path, looking for fractions.

        // Use navigate_trace to build a tree of navigation path items
        // We can clone them, since we aren't modifying them - just checking what they are
        let mut nav_items = vec![];
        self.navigate_trace(
            &mut path.to_navigator(), 
            |item: &mut Node| nav_items.push(item.clone())
        );

        // Iterate reversed, since we're looking from the inside out
        for (i, item) in nav_items.iter().rev().enumerate() {
            // Division is currently the only thing with vertical movement
            if let Node::Divide(top, bottom) = item {
                // Work out the true index of this in the nav tree.
                // Remember, we're going backwards!
                let true_index = (nav_items.len() - i) - 1;

                let (index_allowing_movement, index_to_move_to) = match direction {
                    MoveVerticalDirection::Up => (1, 0),
                    MoveVerticalDirection::Down => (0, 1),
                };

                // Are we on the top?
                if path[true_index] == index_allowing_movement {
                    // Yes!
                    // Determine the index to move to
                    let match_points = renderer.match_vertical_cursor_points(
                        top, bottom, direction
                    );
                    let new_index = match_points[path[true_index + 1]];

                    // Pop up to and including this item, then move to the bottom and the correct
                    // new index
                    path.pop(i + 1);
                    path.push(index_to_move_to);
                    path.push(new_index);
                    break;
                } else {
                    // Keep looking
                }
            }
        }
    }
    
    /// Modifies the given navigation path to move the cursor down.
    pub fn move_down(&mut self, path: &mut NavPath, renderer: &mut impl Renderer) {
        self.move_vertically(path, MoveVerticalDirection::Down, renderer);
    }

    /// Modifies the given navigation path to move the cursor up.
    pub fn move_up(&mut self, path: &mut NavPath, renderer: &mut impl Renderer) {
        self.move_vertically(path, MoveVerticalDirection::Up, renderer);
    }

    /// Inserts the given node at the cursor position, and moves the cursor accordingly.
    pub fn insert(&mut self, path: &mut NavPath, new_node: Node) {
        let (current_node, index) = self.navigate(&mut path.to_navigator());

        current_node.unwrap_unstructured_mut().insert(index, new_node.clone());

        match new_node {
            Node::Sqrt(_) | Node::Divide(_, _) => {
                // Move into the new node
                path.push(0);
                path.push(0);
            },

            // Just move past it
            _ => path.offset(1),
        }
    }

    /// Deletes the item behind the cursor.
    pub fn delete(&mut self, path: &mut NavPath) {
        let (current_node, index) = self.navigate(&mut path.to_navigator());

        if index > 0 {
            // Delete if there is something behind the cursor
            current_node.unwrap_unstructured_mut().remove(index - 1);
            path.offset(-1);
        } else {
            // Are we in a container?
            if !path.root() {
                // Move right and delete, to delete this item
                // (Assumes containers have no horizontal slots)
                self.move_right(path);
                self.delete(path);
            }
        }
    }

    // TODO: tests
    pub fn evaluate(&self) -> Result<Decimal, Box<dyn Error>> {
        match self {
            Node::Number(n) => Ok((*n).into()),
            Node::Sqrt(inner) =>
                inner.evaluate()?.sqrt().ok_or(box MathsError("illegal sqrt".into())),
            Node::Add(a, b) => Ok(a.evaluate()? + b.evaluate()?),
            Node::Subtract(a, b) => Ok(a.evaluate()? - b.evaluate()?),
            Node::Multiply(a, b) => Ok(a.evaluate()? * b.evaluate()?),
            Node::Divide(a, b) => Ok(a.evaluate()? / b.evaluate()?),
            Node::Parentheses(inner) => inner.evaluate(),

            Node::Token(_) | Node::Unstructured(_) =>
                Err(box NodeError("cannot evaluate unstructured nodes".into())),
        }
    }

    /// Panics if this node is not unstructured, and returns the children of
    /// the node.
    pub fn unwrap_unstructured(&self) -> &Vec<Node> {
        if let Node::Unstructured(children) = self {
            children
        } else {
            panic!("expected node to be unstructured")
        }
    }

    /// Panics if this node is not unstructured, and returns the children of
    /// the node.
    pub fn unwrap_unstructured_mut(&mut self) -> &mut Vec<Node> {
        if let Node::Unstructured(children) = self {
            children
        } else {
            panic!("expected node to be unstructured")
        }
    }
}
