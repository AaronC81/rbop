use alloc::{vec::Vec, vec, boxed::Box};
use rust_decimal::Decimal;
use crate::{error::{Error, NodeError}, nav::{NavPath, NavPathNavigator}, render::Renderer};
use super::{parser, structured::StructuredNode};

#[derive(Clone)]
pub enum UnstructuredItem<'a> {
    Node(&'a UnstructuredNode),
    List(&'a UnstructuredNodeList),
}

pub enum MoveVerticalDirection {
    Up,
    Down,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Token {
    Add,
    Subtract,
    Multiply,
    Divide,
    Digit(u8),
}

/// An unstructured node is one which can be inputted by the user. Unstructured nodes have as little
/// structure as possible - for example, "2+3*5" is represented as a flat list of tokens, with no
/// respect for precedence.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum UnstructuredNode {
    Token(Token),
    Sqrt(UnstructuredNodeList),
    Fraction(UnstructuredNodeList, UnstructuredNodeList),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct UnstructuredNodeList {
    items: Vec<UnstructuredNode>
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct UnstructuredNodeRoot {
    root: UnstructuredNodeList
}

trait Navigable {
    /// Given a navigation path, returns the node from following that path, and the index into that
    /// node. The navigation path will always terminate on an unstructured node list, so the final
    /// index in the path will be an index into the unstructured node list's items.
    fn navigate(&mut self, path: &mut NavPathNavigator) -> (&mut UnstructuredNodeList, usize) {
        self.navigate_trace(path, |_| {})
    }

    fn navigate_trace<F>(&mut self, path: &mut NavPathNavigator, trace: F) -> (&mut UnstructuredNodeList, usize) 
        where F : FnMut(&mut UnstructuredItem);
}

impl Navigable for UnstructuredNode {
    fn navigate_trace<F>(&mut self, path: &mut NavPathNavigator, mut trace: F) -> (&mut UnstructuredNodeList, usize) 
        where F : FnMut(&mut UnstructuredItem)
    {
        trace(&mut UnstructuredItem::Node(&mut self));

        if path.here() {
            panic!("navigation path must end on unstructured node");
        }

        let next_index = path.next();
        let step_path = &mut path.step();

        match self {
            UnstructuredNode::Sqrt(inner) => {
                if next_index != 0 {
                    panic!("index out of range for sqrt navigation")
                }

                inner.navigate_trace(step_path, trace)
            },
            UnstructuredNode::Fraction(top, bottom) => {
                if next_index == 0 {
                    top.navigate_trace(step_path, trace)
                } else if next_index == 1 {
                    bottom.navigate_trace(step_path, trace)
                } else {
                    panic!("index out of range for divide navigation")
                }
            },
            UnstructuredNode::Token(_) => panic!("cannot navigate into token"),
        }
    }
}

impl Navigable for UnstructuredNodeList {
    fn navigate_trace<F>(&mut self, path: &mut NavPathNavigator, mut trace: F) -> (&mut UnstructuredNodeList, usize) 
        where F : FnMut(&mut UnstructuredItem)
    {
        trace(&mut UnstructuredItem::List(&mut self));

        if path.here() {
            return (self, path.next());
        }

        self.items[path.next()].navigate_trace(&mut path.step(), trace)
    }
}

impl UnstructuredNodeRoot { 
    /// Modifies the given navigation path to move the cursor right.
    pub fn move_right(&mut self, path: &mut NavPath) {
        // Fetch the node which we're navigating within
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());
        let children = current_node.items;

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
                UnstructuredNode::Sqrt(_) | UnstructuredNode::Fraction(_, _) => {
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
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());
        let children = current_node.items;

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
                UnstructuredNode::Sqrt(n) | UnstructuredNode::Fraction(n, _) => {
                    // Navigate into its first/only slot, and start at the first item of the
                    // unstructured
                    path.push(0);
                    path.push(n.items.len());
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
        self.root.navigate_trace(
            &mut path.to_navigator(), 
            |item: &mut UnstructuredItem| nav_items.push(item.clone())
        );

        // Iterate reversed, since we're looking from the inside out
        for (i, item) in nav_items.iter().rev().enumerate() {
            // Division is currently the only thing with vertical movement
            if let UnstructuredItem::Node(UnstructuredNode::Fraction(top, bottom)) = item {
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
    pub fn insert(&mut self, path: &mut NavPath, new_node: UnstructuredNode) {
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());

        current_node.items.insert(index, new_node.clone());

        match new_node {
            UnstructuredNode::Sqrt(_) | UnstructuredNode::Fraction(_, _) => {
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
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());

        if index > 0 {
            // Delete if there is something behind the cursor
            current_node.items.remove(index - 1);
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
}

/// Implemented by types which can be _upgraded_ - that is, converted into a `StructuredNode`.
pub trait Upgradable {
    fn upgrade(&self) -> Result<StructuredNode, Box<dyn Error>>;
}

impl Upgradable for UnstructuredNodeList {
    fn upgrade(&self) -> Result<StructuredNode, Box<dyn Error>> {
        parser::Parser {
            index: 0,
            nodes: &self.items[..]
        }.parse()
    }
}

impl Upgradable for UnstructuredNode {
    fn upgrade(&self) -> Result<StructuredNode, Box<dyn Error>> {
        match self {
            UnstructuredNode::Sqrt(inner)
                => Ok(StructuredNode::Sqrt(box inner.upgrade()?)),

            UnstructuredNode::Fraction(a, b)
                => Ok(StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?)),

            UnstructuredNode::Token(_) => Err(box NodeError("token cannot be upgraded".into())),
        }
    }
}

