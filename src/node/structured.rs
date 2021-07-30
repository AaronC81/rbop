use core::cmp::max;

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use rust_decimal::{Decimal, MathematicalOps};

use crate::error::{Error, MathsError};
use crate::render::{Glyph, LayoutBlock, Layoutable, MergeBaseline, Renderer};
use crate::nav::NavPathNavigator;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum StructuredNode {
    Number(Decimal),
    Sqrt(Box<StructuredNode>),
    Add(Box<StructuredNode>, Box<StructuredNode>),
    Subtract(Box<StructuredNode>, Box<StructuredNode>),
    Multiply(Box<StructuredNode>, Box<StructuredNode>),
    Divide(Box<StructuredNode>, Box<StructuredNode>),
    Parentheses(Box<StructuredNode>),
}

impl StructuredNode {
    /// Returns true if this node is `Add` or `Subtract`.
    pub fn add_or_sub(&self) -> bool {
        matches!(&self, StructuredNode::Add(_, _) | StructuredNode::Subtract(_, _))
    }

    /// Returns true if this node is `Multiply` or `Divide`.
    pub fn mul_or_div(&self) -> bool {
        matches!(&self, StructuredNode::Multiply(_, _) | StructuredNode::Divide(_, _))
    }

    /// Returns a clone of this node wrapped in `Parentheses`.
    pub fn in_parentheses(&self) -> StructuredNode {
        StructuredNode::Parentheses(box self.clone())
    }

    /// If `parens` is true, returns a clone of this node wrapped in `Parentheses`, otherwise just
    /// returns a plain clone of this node.
    pub fn in_parentheses_or_clone(&self, parens: bool) -> StructuredNode {
        if parens {
            self.in_parentheses()
        } else {
            self.clone()
        }
    }

    /// Returns a clone of this node tree with added parentheses to show the order of operations
    /// when the tree is rendered.
    /// The tree should be upgraded before doing this.
    pub fn disambiguate(&self) -> Result<StructuredNode, Box<dyn Error>> {
        Ok(match self {
            // We need to add parentheses around:
            //   - operations which mix precedence, e.g. (3+2)*4
            //   - operations which go against standard associativity for - and /, e.g. 3-(3-2)

            StructuredNode::Multiply(l, r) => {
                let l = l.in_parentheses_or_clone(l.add_or_sub());
                let r = r.in_parentheses_or_clone(r.add_or_sub() || r.mul_or_div());
                StructuredNode::Multiply(box l, box r)
            }
            StructuredNode::Divide(l, r) => {
                let l = l.in_parentheses_or_clone(l.add_or_sub());
                let r = r.in_parentheses_or_clone(r.add_or_sub() || r.mul_or_div());
                StructuredNode::Divide(box l, box r)
            }

            StructuredNode::Add(l, r) => {
                let r = r.in_parentheses_or_clone(r.add_or_sub());
                StructuredNode::Add(l.clone(), box r)
            }
            StructuredNode::Subtract(l, r) => {
                let r = r.in_parentheses_or_clone(r.add_or_sub());
                StructuredNode::Subtract(l.clone(), box r)
            }

            StructuredNode::Number(_) | StructuredNode::Sqrt(_) | StructuredNode::Parentheses(_)
                => self.clone(),
        })
    }

    /// Evaluates this node into a single number.
    pub fn evaluate(&self) -> Result<Decimal, Box<dyn Error>> {
        match self {
            StructuredNode::Number(n) => Ok((*n).into()),
            StructuredNode::Sqrt(inner) =>
                inner.evaluate()?.sqrt().ok_or(box MathsError("illegal sqrt".into())),
            StructuredNode::Add(a, b) => Ok(a.evaluate()? + b.evaluate()?),
            StructuredNode::Subtract(a, b) => Ok(a.evaluate()? - b.evaluate()?),
            StructuredNode::Multiply(a, b) => Ok(a.evaluate()? * b.evaluate()?),
            StructuredNode::Divide(a, b) => Ok(a.evaluate()? / b.evaluate()?),
            StructuredNode::Parentheses(inner) => inner.evaluate(),
        }
    }
}

/// Calculates layout for a binop, with the operator being the `glyph`.
fn layout_binop(renderer: &mut impl Renderer, glyph: Glyph, left: &StructuredNode, right: &StructuredNode) -> LayoutBlock {
    // These are structured nodes, which (currently) never have a cursor

    let left_layout = left.layout(renderer, None);
    let binop_layout = LayoutBlock::from_glyph(renderer, glyph)
        .move_right_of_other(renderer, &left_layout);
    let right_layout = right.layout(renderer, None)
        .move_right_of_other(renderer, &binop_layout);

    left_layout
        .merge_along_baseline(&binop_layout)
        .merge_along_baseline(&right_layout)
}

impl Layoutable for StructuredNode {
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock {
        match self {
            StructuredNode::Number(number) => {
                // We'll worry about negatives later!
                if *number < Decimal::ZERO { panic!("negative numbers not supported") }

                let glyph_layouts = (*number)
                    .to_string()
                    .chars()
                    .map(|c| Glyph::Digit { number: c.to_digit(10).unwrap() as u8 })
                    .map(|g| LayoutBlock::from_glyph(renderer, g))
                    .collect::<Vec<_>>();

                LayoutBlock::layout_horizontal(renderer, &glyph_layouts[..])
            },

            StructuredNode::Add(left, right) => layout_binop(renderer, Glyph::Add, left, right),
            StructuredNode::Subtract(left, right) => layout_binop(renderer, Glyph::Subtract, left, right),
            StructuredNode::Multiply(left, right) => layout_binop(renderer, Glyph::Multiply, left, right),

            StructuredNode::Divide(top, bottom) => {
                let (mut top_path, mut bottom_path) = {
                    if let Some(p) = path {
                        if p.next() == 0 {
                            (Some(p.step()), None)
                        } else if p.next() == 1 {
                            (None, Some(p.step()))
                        } else {
                            panic!()
                        }
                    } else {
                        (None, None)
                    }
                };

                let top_layout = top.layout(
                    renderer,
                    (&mut top_path).as_mut()
                );
                let bottom_layout = bottom.layout(
                    renderer,
                    (&mut bottom_path).as_mut()
                );

                // The fraction line should be the widest of the two
                let line_width = max(
                    top_layout.area(renderer).width,
                    bottom_layout.area(renderer).width,
                );
                let line_layout = LayoutBlock::from_glyph(renderer, Glyph::Fraction {
                    inner_width: line_width
                }).move_below_other(renderer, &top_layout);

                let bottom_layout = bottom_layout
                    .move_below_other(renderer, &line_layout);

                top_layout
                    .merge_along_vertical_centre(renderer, &line_layout, MergeBaseline::OtherAsBaseline)
                    .merge_along_vertical_centre(renderer, &bottom_layout, MergeBaseline::SelfAsBaseline)
            }

            StructuredNode::Sqrt(inner) => {
                // Lay out the inner item first
                let mut path = if let Some(p) = path {
                    if p.next() == 0 {
                        Some(p.step())
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let inner_layout = inner.layout(renderer, (&mut path).as_mut());
                let inner_area = inner_layout.area(renderer);

                // Get glyph size for the sqrt symbol
                let sqrt_symbol_layout = LayoutBlock::from_glyph(renderer, Glyph::Sqrt {
                    inner_area
                });

                // We assume that the inner layout goes in the very bottom right, so work out the
                // offset required based on the difference of the two areas
                let x_offset = sqrt_symbol_layout.area(renderer).width - inner_layout.area(renderer).width;
                let y_offset = sqrt_symbol_layout.area(renderer).height - inner_layout.area(renderer).height;

                // Merge the two
                sqrt_symbol_layout.merge_in_place(
                    renderer, 
                    &inner_layout.offset(x_offset, y_offset),
                    MergeBaseline::OtherAsBaseline
                )
            }

            StructuredNode::Parentheses(_) => todo!(),
        }
    }
}
