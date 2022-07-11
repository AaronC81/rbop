//! Implements [Layoutable] for unstructured nodes, enabling them to be [rendered](crate::render).

use alloc::{vec::Vec, vec};

use crate::{render::{Layoutable, Renderer, LayoutComputationProperties, LayoutBlock, Glyph}, UnstructuredNodeRoot, nav::NavPathNavigator, UnstructuredNode, node::common, UnstructuredNodeList, UnstructuredItem};

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
