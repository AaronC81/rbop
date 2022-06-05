use core::iter::repeat;

use alloc::{vec::Vec, vec};
use crate::{error::NodeError, nav::{self, MoveVerticalDirection, NavPath, NavPathNavigator}, render::{Glyph, LayoutBlock, Layoutable, Renderer, Viewport, ViewportVisibility, LayoutComputationProperties, CalculatedPoint}};
use super::{common, parser, structured::StructuredNode, function::Function};

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
            UnstructuredNode::Power(exp) => {
                if next_index != 0 {
                    panic!("index out of range for power navigation")
                }

                exp.navigate_trace(step_path, trace)
            }
            UnstructuredNode::FunctionCall(_, args) => {
                if next_index >= args.len() {
                    panic!("index out of range for function call navigation")
                }

                args[next_index].navigate_trace(step_path, trace)
            }
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
                // Are we inside a function call?
                // To check, clone the path, step out to the structural node, and navigate to it
                // Sure, there can be sub-nodes (like we need to consider for fractions), but we'll only
                // ever want to hop between arguments if we're outside those, so this relatively naive
                // check is fine
                let mut outer_path = path.clone();
                outer_path.pop(2);
                let (outer_node, index) = self.root.navigate(&mut outer_path.to_navigator());
                if let UnstructuredNode::FunctionCall(_, args) = &outer_node.items[index] {
                    // Can we move right into another argument?
                    let current_arg_index = path[path.len() - 2];
                    if current_arg_index < args.len() - 1 {
                        // Yes, we can! Move right into the beginning of the next argument
                        path.pop(2);
                        path.push(current_arg_index + 1);
                        path.push(0);
                        return
                    }
                }

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
                UnstructuredNode::Sqrt(_) | UnstructuredNode::Fraction(_, _) | UnstructuredNode::Parentheses(_) | UnstructuredNode::Power(_) | UnstructuredNode::FunctionCall(_, _) => {
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
                // Are we inside a function call?
                // (Logic largely duplicated from move_right)
                let mut outer_path = path.clone();
                outer_path.pop(2);
                let (outer_node, index) = self.root.navigate(&mut outer_path.to_navigator());
                if let UnstructuredNode::FunctionCall(_, args) = &outer_node.items[index] {
                    // Can we move right into another argument?
                    let current_arg_index = path[path.len() - 2];
                    if current_arg_index > 0 {
                        // Yes, we can! Move right into the end of the next argument
                        path.pop(2);
                        path.push(current_arg_index - 1);
                        path.push(args[current_arg_index - 1].items.len());
                        return
                    }
                }

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
                UnstructuredNode::Sqrt(n) | UnstructuredNode::Fraction(n, _) | UnstructuredNode::Parentheses(n) | UnstructuredNode::Power(n) => {
                    // Navigate into its first/only slot, and start at the last item of the
                    // unstructured
                    path.push(0);
                    path.push(n.items.len());
                },

                UnstructuredNode::FunctionCall(_, args) => {
                    // Move into last slot
                    // TODO: won't work with zero-argument functions
                    path.push(args.len() - 1);
                    path.push(args.last().expect("no args in call").items.len());
                }

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

        let mut moved_within = false;

        // Iterate reversed, since we're looking from the inside out
        for (i, ri, item) in self.nav_nodes_outwards(path) {
            // Division is currently the only thing with vertical movement
            if let UnstructuredNode::Fraction(ref top, ref bottom) = item {
                let (index_allowing_movement, index_to_move_to) = match direction {
                    MoveVerticalDirection::Up => (1, 0),
                    MoveVerticalDirection::Down => (0, 1),
                };

                // Are we on the top?
                if path[i] == index_allowing_movement {
                    // Yes!
                    // Determine the index to move to
                    let match_points = nav::match_vertical_cursor_points(
                        renderer, top, bottom, direction
                    );
                    let new_index = match_points[path[i + 1]];

                    // Pop up to and including this item, then move to the bottom and the correct
                    // new index
                    path.pop(ri + 1);
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
            UnstructuredNode::Sqrt(_) | UnstructuredNode::Fraction(_, _) | UnstructuredNode::Parentheses(_) | UnstructuredNode::Power(_) | UnstructuredNode::FunctionCall(_, _) => {
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

    /// Clears the entire node structure, resetting the viewport and cursor.
    pub fn clear(&mut self, path: &mut NavPath, _renderer: &mut impl Renderer, mut viewport: Option<&mut Viewport>) {
        // Delete everything!
        self.root.items = vec![];

        // Reset cursor and viewport
        *path = NavPath::new(vec![0]);
        viewport.as_mut().map(|x| x.offset = CalculatedPoint { x: 0, y: 0 });
    }


    /// Builds a list of the items at each element of the nav path.
    ///
    /// Each index in the returned vec has a direct mapping to each index in the nav path. If the
    /// item in the returned vec is None, the nav path item is not a node. If it is Some, the
    /// wrapped node is the node at that index in the nav path.
    ///
    /// The returned nodes are clones, not references, so modifying them will not affect the node
    /// tree.
    fn nav_node_list(&mut self, path: &mut NavPath) -> Vec<Option<UnstructuredNode>> {
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
        nav_items
    }

    /// Builds a list of the nodes at each element of the nav path, working outwards from the node
    /// which contains the cursor.
    ///
    /// The returned vec items are of the form (nav list index, reverse nav list index, node). Since
    /// the list works outwards, the nav list indexes are strictly decreasing. The reverse indexes
    /// start from the beginning of the nav list instead and are strictly increasing.
    ///
    /// The returned nodes are clones, not references, so modifying them will not affect the node
    /// tree.
    fn nav_nodes_outwards(&mut self, path: &mut NavPath) -> Vec<(usize, usize, UnstructuredNode)> {
        let mut result = vec![];

        // Get items
        let nav_items = self.nav_node_list(path);
        let nav_items_len = nav_items.len();

        // Reverse node nav list, so we iterate from the inside out
        for (i, item) in nav_items.into_iter().rev().enumerate() {
            // If this node is actually a node...
            if let Some(node) = item {
                // Work out the true index of this in the nav tree.
                // Remember, we're going backwards!
                let true_index = (nav_items_len - i) - 1;

                // Yield
                result.push((true_index, i, node));
            }
        }

        result
    }
}

/// Implemented by types which can be _upgraded_ - that is, converted into a `StructuredNode`.
pub trait Upgradable {
    fn upgrade(&self) -> Result<StructuredNode, NodeError>;
}

impl Upgradable for UnstructuredNodeList {
    fn upgrade(&self) -> Result<StructuredNode, NodeError> {
        parser::Parser {
            index: 0,
            nodes: &self.items[..]
        }.parse()
    }
}

impl Upgradable for UnstructuredNodeRoot {
    fn upgrade(&self) -> Result<StructuredNode, NodeError> {
        self.root.upgrade()
    }
}

impl Upgradable for UnstructuredNode {
    fn upgrade(&self) -> Result<StructuredNode, NodeError> {
        match self {
            UnstructuredNode::Sqrt(inner)
                => Ok(StructuredNode::Sqrt(box inner.upgrade()?)),

            UnstructuredNode::Parentheses(inner)
                => Ok(StructuredNode::Parentheses(box inner.upgrade()?)),

            UnstructuredNode::Fraction(a, b)
                => Ok(StructuredNode::Divide(box a.upgrade()?, box b.upgrade()?)),

            // Parser should always handle this
            UnstructuredNode::Power(_)
                => Err(NodeError::PowerMissingBase),

            UnstructuredNode::FunctionCall(func, args)
                => Ok(StructuredNode::FunctionCall(*func, 
                    args.iter().map(|a| a.upgrade()).collect::<Result<Vec<_>, _>>()?
                )),

            UnstructuredNode::Token(_) => Err(NodeError::CannotUpgradeToken),
        }
    }
}

impl Layoutable for UnstructuredNodeRoot {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> LayoutBlock {
        self.root.layout(renderer, path, properties)
    }
}

impl Layoutable for UnstructuredNode {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> crate::render::LayoutBlock {
        match self {
            UnstructuredNode::Token(token)
                => LayoutBlock::from_glyph(renderer, (*token).into(), properties),

            UnstructuredNode::Sqrt(inner)
                => common::layout_sqrt(inner, renderer, path, properties),
            UnstructuredNode::Fraction(top, bottom)
                => common::layout_fraction(top, bottom, renderer, path, properties),
            UnstructuredNode::Parentheses(inner)
                => common::layout_parentheses(inner, renderer, path, properties),
            UnstructuredNode::Power(exp)
                => common::layout_power(None, exp, renderer, path, properties),
            UnstructuredNode::FunctionCall(func, args)
                => common::layout_function_call(*func, args, renderer, path, properties),
        }
    }
}

impl Layoutable for UnstructuredNodeList {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> LayoutBlock {
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
                (&mut paths[i]).as_mut(),
                properties,
            ))
            .collect::<Vec<_>>();

        // If the cursor is here, insert it
        if let Some(idx) = cursor_insertion_index {
            // Get the layout to match the size to
            let temp_layout;
            let cursor_match_layout = if layouts.is_empty() {
                // Our default size will be that of the digit 0
                temp_layout = Some(LayoutBlock::from_glyph(renderer, Glyph::Digit {
                    number: 0
                }, properties));
                &temp_layout.as_ref().unwrap()
            } else if idx == 0 {
                &layouts[idx]
            } else if idx == layouts.len() {
                &layouts[idx - 1]
            } else {
                let after = &layouts[idx];
                let before = &layouts[idx - 1];

                if after.area.height > before.area.height {
                    after
                } else {
                    before
                }
            };
            let cursor_height = cursor_match_layout.area.height;
            let cursor_baseline = cursor_match_layout.baseline;

            // Hackily match the baseline
            let mut cursor_layout = LayoutBlock::from_glyph(renderer, Glyph::Cursor {
                height: cursor_height,
            }, properties);
            cursor_layout.baseline = cursor_baseline;

            layouts.insert(idx, cursor_layout)
        }

        // If the list is still empty (i.e. this list was empty anyway, and the cursor's not in it)
        // then insert a placeholder
        if layouts.is_empty() {
            layouts.push(LayoutBlock::from_glyph(renderer, Glyph::Placeholder, properties))
        }

        LayoutBlock::layout_horizontal(&layouts[..])

    }
}

impl<'a> Layoutable for UnstructuredItem<'a> {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> crate::render::LayoutBlock {
        match self {
            UnstructuredItem::Node(node) => node.layout(renderer, path, properties),
            UnstructuredItem::List(children) => children.layout(renderer, path, properties),
        }
    }
}

pub trait Serializable where Self: Sized {
    fn serialize(&self) -> Vec<u8>;

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self>;
}

impl<T : num_traits::PrimInt> Serializable for T {
    // Numbers are only used for counting here, and it's quite unlikely we'll get any over a byte
    // large. So, numbers are serialized using a funky variable-length representation:
    //   0x02 = 2
    //   0xFE = 254
    //   0xFF 0x00 = 255
    //   0xFF 0x02 = 257
    //   0xFF 0xFF 0x02 = 512
    // 0xFF is always followed by another byte which is added to the 0xFF.
    fn serialize(&self) -> Vec<u8> {
        if self < &Self::zero() { panic!("cannot serialize negative numbers"); }

        let mut result = vec![];
        let mut current = *self;
        while current >= Self::from(0xFF).unwrap() {
            current = current - Self::from(0xFF).unwrap();
            result.push(0xFF);
        }
        result.push(num_traits::cast(current).unwrap());

        result
    }

    fn deserialize(bytes: &mut dyn Iterator<Item = u8>) -> Option<Self> {
        let mut result = Self::zero();

        loop {
            let byte = bytes.next()?;
            result = result + Self::from(byte).unwrap();
            if byte != 0xFF { break; }
        }

        Some(result)
    }
}

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
