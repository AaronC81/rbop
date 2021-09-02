use core::cmp::max;

use alloc::{vec::Vec, vec, boxed::Box};
use rust_decimal::Decimal;
use crate::{error::{Error, NodeError}, nav::{self, MoveVerticalDirection, NavPath, NavPathNavigator}, render::{Glyph, LayoutBlock, Layoutable, MergeBaseline, Renderer, Viewport, ViewportVisibility}};
use super::{common, parser, structured::StructuredNode};

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

/// An unstructured node is one which can be inputted by the user. Unstructured nodes have as little
/// structure as possible - for example, "2+3*5" is represented as a flat list of tokens, with no
/// respect for precedence.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum UnstructuredNode {
    Token(Token),
    Sqrt(UnstructuredNodeList),
    Fraction(UnstructuredNodeList, UnstructuredNodeList),
    Parentheses(UnstructuredNodeList),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct UnstructuredNodeList {
    pub items: Vec<UnstructuredNode>
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct UnstructuredNodeRoot {
    pub root: UnstructuredNodeList
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MoveResult {
    MovedWithin,
    MovedOut,
}

pub trait Navigable {
    /// Given a navigation path, returns the node from following that path, and the index into that
    /// node. The navigation path will always terminate on an unstructured node list, so the final
    /// index in the path will be an index into the unstructured node list's items.
    fn navigate(&mut self, path: &mut NavPathNavigator) -> (&mut UnstructuredNodeList, usize) {
        self.navigate_trace(path, |_| {})
    }

    fn navigate_trace<F>(&mut self, path: &mut NavPathNavigator, trace: F) -> (&mut UnstructuredNodeList, usize) 
        where F : FnMut(UnstructuredItem);
}

impl Navigable for UnstructuredNode {
    fn navigate_trace<F>(&mut self, path: &mut NavPathNavigator, mut trace: F) -> (&mut UnstructuredNodeList, usize) 
        where F : FnMut(UnstructuredItem)
    {
        trace(UnstructuredItem::Node(&self.clone()));

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
            UnstructuredNode::Parentheses(inner) => {
                if next_index != 0 {
                    panic!("index out of range for parens navigation")
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
        where F : FnMut(UnstructuredItem)
    {
        trace(UnstructuredItem::List(&self.clone()));

        if path.here() {
            return (self, path.next());
        }

        self.items[path.next()].navigate_trace(&mut path.step(), trace)
    }
}

impl UnstructuredNodeRoot { 
    /// Checks if the cursor is outside of the viewport. If so, moves the viewport to fit it inside
    /// again.
    pub fn ensure_cursor_visible(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, viewport: Option<&mut Viewport>) {
        if let Some(viewport) = viewport {
            let cursor_visibility = renderer.cursor_visibility(
                self,
                &mut path.to_navigator(),
                Some(&*viewport),
            );

            if let ViewportVisibility::Clipped { top_clip, bottom_clip, left_clip, right_clip, .. } = cursor_visibility {
                match (top_clip, bottom_clip) {
                    (0, 0) => (),
                    (_, 0) => viewport.offset.y -= top_clip,
                    (0, _) => viewport.offset.y += bottom_clip,
                    _ => panic!("cursor does not fit vertically in viewport"),
                }

                match (left_clip, right_clip) {
                    (0, 0) => (),
                    (_, 0) => viewport.offset.x -= left_clip,
                    (0, _) => viewport.offset.x += right_clip,
                    _ => panic!("cursor does not fit horizontally in viewport"),
                }
            }
        }
    }

    /// Modifies the given navigation path to move the cursor right.
    pub fn move_right(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, viewport: Option<&mut Viewport>) {
        // Fetch the node which we're navigating within
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());
        let children = &current_node.items;

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
                UnstructuredNode::Sqrt(_) | UnstructuredNode::Fraction(_, _) | UnstructuredNode::Parentheses(_) => {
                    // Navigate into its first/only slot, and start at the first item of the
                    // unstructured
                    path.push(0);
                    path.push(0);
                },

                // Token, we can just move past it
                UnstructuredNode::Token(_) => path.offset(1),
            }
        }

        self.ensure_cursor_visible(path, renderer, viewport);
    }

    /// Modifies the given navigation path to move the cursor left.
    pub fn move_left(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, viewport: Option<&mut Viewport>) {
        // Fetch the node which we're navigating within
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());
        let children = &current_node.items;

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
                UnstructuredNode::Sqrt(n) | UnstructuredNode::Fraction(n, _) | UnstructuredNode::Parentheses(n) => {
                    // Navigate into its first/only slot, and start at the first item of the
                    // unstructured
                    path.push(0);
                    path.push(n.items.len());
                },

                // Anything else, nothing special needed
                UnstructuredNode::Token(_) => (),
            }
        }

        self.ensure_cursor_visible(path, renderer, viewport);
    }

    fn move_vertically(
        &mut self,
        path: &mut NavPath,
        direction: MoveVerticalDirection,
        renderer: &mut impl Renderer,
        viewport: Option<&mut Viewport>
    ) -> MoveResult {
        // Say you're in a sqrt at the top of a fraction, and you press down, you'd expect it to
        // move to the bottom of the fraction.
        // That's why we need to check up the entire nav path, looking for fractions.

        // Use navigate_trace to build a tree of navigation path items
        // We can clone them, since we aren't modifying them - just checking what they are
        let mut nav_items = vec![];
        self.root.navigate_trace(
            &mut path.to_navigator(), 
            |item| {
                // I fought the borrow checker and lost :(
                // We only care about nodes, so this makes our life easier
                // We still want nav_items to be the right length
                if let UnstructuredItem::Node(node) = item {
                    nav_items.push(Some(node.clone()));
                } else {
                    nav_items.push(None);
                }
            }
        );

        let mut moved_within = false;

        // Iterate reversed, since we're looking from the inside out
        for (i, item) in nav_items.iter().rev().enumerate() {
            // Division is currently the only thing with vertical movement
            if let Some(UnstructuredNode::Fraction(top, bottom)) = item {
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
                    let match_points = nav::match_vertical_cursor_points(
                        renderer, top, bottom, direction
                    );
                    let new_index = match_points[path[true_index + 1]];

                    // Pop up to and including this item, then move to the bottom and the correct
                    // new index
                    path.pop(i + 1);
                    path.push(index_to_move_to);
                    path.push(new_index);
                    moved_within = true;
                    break;
                } else {
                    // Keep looking
                }
            }
        }

        self.ensure_cursor_visible(path, renderer, viewport);

        if moved_within {
            MoveResult::MovedWithin
        } else {
            MoveResult::MovedOut
        }
    }
    
    /// Modifies the given navigation path to move the cursor down.
    pub fn move_down(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, viewport: Option<&mut Viewport>) -> MoveResult {
        self.move_vertically(path, MoveVerticalDirection::Down, renderer, viewport)
    }

    /// Modifies the given navigation path to move the cursor up.
    pub fn move_up(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, viewport: Option<&mut Viewport>) -> MoveResult {
        self.move_vertically(path, MoveVerticalDirection::Up, renderer, viewport)
    }

    /// Inserts the given node at the cursor position, and moves the cursor accordingly.
    pub fn insert(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, viewport: Option<&mut Viewport>, new_node: UnstructuredNode) {
        let (current_node, index) = self.root.navigate(&mut path.to_navigator());

        current_node.items.insert(index, new_node.clone());

        match new_node {
            UnstructuredNode::Sqrt(_) | UnstructuredNode::Fraction(_, _) | UnstructuredNode::Parentheses(_) => {
                // Move into the new node
                path.push(0);
                path.push(0);
            },

            // Just move past it
            UnstructuredNode::Token(_) => path.offset(1),
        }

        self.ensure_cursor_visible(path, renderer, viewport);
    }

    /// Deletes the item behind the cursor.
    pub fn delete(&mut self, path: &mut NavPath, renderer: &mut impl Renderer, mut viewport: Option<&mut Viewport>) {
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
                self.move_right(path, renderer, viewport.as_mut().map(|x| x as _));
                self.delete(path, renderer, viewport.as_mut().map(|x| x as _));
            }
        }

        self.ensure_cursor_visible(path, renderer, viewport.as_mut().map(|x| x as _));
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

impl Upgradable for UnstructuredNodeRoot {
    fn upgrade(&self) -> Result<StructuredNode, Box<dyn Error>> {
        self.root.upgrade()
    }
}

impl Upgradable for UnstructuredNode {
    fn upgrade(&self) -> Result<StructuredNode, Box<dyn Error>> {
        match self {
            UnstructuredNode::Sqrt(inner)
                => Ok(StructuredNode::Sqrt(box inner.upgrade()?)),

            UnstructuredNode::Parentheses(inner)
                => Ok(StructuredNode::Parentheses(box inner.upgrade()?)),

            UnstructuredNode::Fraction(a, b)
                => Ok(StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?)),

            UnstructuredNode::Token(_) => Err(box NodeError("token cannot be upgraded".into())),
        }
    }
}

impl Layoutable for UnstructuredNodeRoot {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock {
        self.root.layout(renderer, path)
    }
}

impl Layoutable for UnstructuredNode {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> crate::render::LayoutBlock {
        match self {
            UnstructuredNode::Token(token)
                => LayoutBlock::from_glyph(renderer, (*token).into()),

            UnstructuredNode::Sqrt(inner)
                => common::layout_sqrt(inner, renderer, path),
            UnstructuredNode::Fraction(top, bottom)
                => common::layout_fraction(top, bottom, renderer, path),
            UnstructuredNode::Parentheses(inner)
                => common::layout_parentheses(inner, renderer, path),
        }
    }
}

impl Layoutable for UnstructuredNodeList {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock {
        let children = &self.items;

        // We never actually mutate the paths...
        // Unsafe time!
        let mut paths = vec![];
        let mut cursor_insertion_index = None;

        unsafe {
            if let Some(p) = path {
                let p = p as *mut NavPathNavigator;
                for i in 0..children.len() {
                    paths.push({
                        if p.as_mut().unwrap().next() == i && !p.as_mut().unwrap().here() {
                            // The cursor is within the child
                            Some(p.as_mut().unwrap().step())
                        } else {
                            None
                        }
                    })
                }

                // Is the cursor in this element?
                if p.as_mut().unwrap().here() {
                    cursor_insertion_index = Some(p.as_mut().unwrap().next());
                }
            } else {
                for _ in 0..children.len() {
                    paths.push(None);
                }
            }
        }

        let mut layouts = children
            .iter()
            .enumerate()
            .map(|(i, node)| node.layout(
                renderer,
                (&mut paths[i]).as_mut()
            ))
            .collect::<Vec<_>>();

        // If the cursor is here, insert it
        if let Some(idx) = cursor_insertion_index {
            // Get the layout to match the size to
            let mut temp_layout = None;
            let cursor_match_layout = if layouts.is_empty() {
                // Our default size will be that of the digit 0
                temp_layout = Some(LayoutBlock::from_glyph(renderer, Glyph::Digit {
                    number: 0
                }));
                &temp_layout.as_ref().unwrap()
            } else if idx == 0 {
                &layouts[idx]
            } else if idx == layouts.len() {
                &layouts[idx - 1]
            } else {
                let after = &layouts[idx];
                let before = &layouts[idx - 1];

                if after.area(renderer).height > before.area(renderer).height {
                    after
                } else {
                    before
                }
            };
            let cursor_height = cursor_match_layout.area(renderer).height;
            let cursor_baseline = cursor_match_layout.baseline;

            // Hackily match the baseline
            let mut cursor_layout = LayoutBlock::from_glyph(renderer, Glyph::Cursor {
                height: cursor_height,
            });
            cursor_layout.baseline = cursor_baseline;

            layouts.insert(idx, cursor_layout)
        }

        LayoutBlock::layout_horizontal(renderer, &layouts[..])

    }
}

impl<'a> Layoutable for UnstructuredItem<'a> {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> crate::render::LayoutBlock {
        match self {
            UnstructuredItem::Node(node) => node.layout(renderer, path),
            UnstructuredItem::List(children) => children.layout(renderer, path),
        }
    }
}
