use std::{alloc::Layout, cmp::max, unimplemented};
use crate::Token;

use crate::Node;
use crate::nav::NavPathNavigator;

pub type Dimension = u64;

/// Used while the layout is still being calculated, where elements may be before/above the baseline
/// and thus be at negative points.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct CalculatingPoint {
    x: i64,
    y: i64,
}

impl CalculatingPoint {
    pub fn dx(&self, delta: i64) -> CalculatingPoint {
        CalculatingPoint { x: self.x + delta, y: self.y }
    }

    pub fn dy(&self, delta: i64) -> CalculatingPoint {
        CalculatingPoint { x: self.x, y: self.y + delta }
    }
}

/// Used when the layout has been calculated, after elements have been shifted from their
/// `CalculatingPoint`s to be relative to (0, 0).
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct CalculatedPoint {
    pub x: u64,
    pub y: u64,
}

impl CalculatedPoint {
    pub fn dx(&self, delta: i64) -> CalculatedPoint {
        CalculatedPoint { x: (self.x as i64 + delta) as u64, y: self.y }
    }

    pub fn dy(&self, delta: i64) -> CalculatedPoint {
        CalculatedPoint { x: self.x, y: (self.y as i64 + delta) as u64 }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Area {
    pub width: Dimension,
    pub height: Dimension,
}

impl Area {
    pub fn new(width: Dimension, height: Dimension) -> Area {
        Area { width, height }
    }

    pub fn square(size: Dimension) -> Area {
        Area { width: size, height: size }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Glyph {
    Digit { number: u8 },
    
    Add,
    Subtract,
    Multiply,
    Divide,

    Fraction { inner_width: Dimension },

    LeftParenthesis { inner_height: Dimension },
    RightParenthesis { inner_height: Dimension },

    Sqrt { inner_area: Area },

    Cursor { height: Dimension },
}

impl From<Token> for Glyph {
    fn from(token: Token) -> Self {
        match token {
            Token::Add => Glyph::Add,
            Token::Subtract => Glyph::Subtract,
            Token::Multiply => Glyph::Multiply,
            Token::Divide => Glyph::Divide,
            Token::Digit(d) => Glyph::Digit { number: d },
        }
    }
}

#[derive(Debug)]
pub struct LayoutBlock {
    pub glyphs: Vec<(Glyph, CalculatedPoint)>,
    pub baseline: Dimension,
}

pub enum MergeBaseline {
    SelfAsBaseline,
    OtherAsBaseline,
}

impl LayoutBlock {
    fn empty() -> LayoutBlock {
        LayoutBlock { glyphs: vec![], baseline: 0 }
    }

    /// Creates a new layout block with one glyph at the origin. The baseline is the centre of this
    /// glyph.
    fn from_glyph(renderer: &mut impl Renderer, glyph: Glyph) -> LayoutBlock {
        LayoutBlock {
            glyphs: vec![(glyph, CalculatedPoint { x: 0, y: 0 })],
            baseline: renderer.size(glyph).height / 2,
        }
    }

    fn area(&self, renderer: &mut impl Renderer) -> Area {
        let mut width = 0;
        let mut height = 0;

        for (glyph, point) in &self.glyphs {
            let size = renderer.size(*glyph);
            let ex = point.x + size.width;
            let ey = point.y + size.height;
            if ex > width { width = ex }
            if ey > height { height = ey }
        }

        Area { width, height }
    }

    fn offset(&self, dx: Dimension, dy: Dimension) -> LayoutBlock {
        LayoutBlock {
            glyphs: self.glyphs
                .iter()
                .map(|(g, p)| (*g, p.dx(dx as i64).dy(dy as i64)))
                .collect(),
            baseline: self.baseline + dy,
        }
    }

    fn merge_along_baseline(&self, other: &LayoutBlock) -> LayoutBlock {
        // Whose baseline is greater?
        // The points can't go negative, so we'll add to the glyphs of the lesser-baselined layout
        // block
        let (lesser_baselined, greater_baselined) = if self.baseline < other.baseline {
            (self, other)
        } else {
            (other, self)
        };

        let baseline_difference = greater_baselined.baseline - lesser_baselined.baseline;

        let glyphs =
            // Re-align the lesser-baselined glyphs
            lesser_baselined.glyphs
            .iter()
            .cloned()
            .map(|(g, p)| (g, p.dy(baseline_difference as i64)))
            // Chain with the unmodified greater-baselined glyphs
            .chain(greater_baselined.glyphs.iter().cloned())
            .collect::<Vec<_>>();

        LayoutBlock {
            glyphs,
            baseline: greater_baselined.baseline,
        }
    }

    /// Merges the glyphs of two layout blocks along their vertical centre.
    fn merge_along_vertical_centre(&self, renderer: &mut impl Renderer, other: &LayoutBlock, baseline: MergeBaseline) -> LayoutBlock {
        // Whose is wider? (i.e., who has the greatest vertical centre)
        // The points can't go negative, so we'll add to the glyphs of the smaller layout block
        let self_centre = self.area(renderer).width / 2;
        let other_centre = other.area(renderer).width / 2;
        let (thinner, thinner_centre, wider, wider_centre) = if self_centre < other_centre {
            (self, self_centre, other, other_centre)
        } else {
            (other, other_centre, self, self_centre)
        };

        let centre_difference = wider_centre - thinner_centre;

        let glyphs =
            // Re-align the lesser-baselined glyphs
            thinner.glyphs
            .iter()
            .cloned()
            .map(|(g, p)| (g, p.dx(centre_difference as i64)))
            // Chain with the unmodified greater-baselined glyphs
            .chain(wider.glyphs.iter().cloned())
            .collect::<Vec<_>>();

        LayoutBlock {
            glyphs,
            baseline: match baseline {
                MergeBaseline::SelfAsBaseline => self.baseline,
                MergeBaseline::OtherAsBaseline => other.baseline,
            },
        }
    }

    /// Assuming that two layout blocks start at the same point, returns a clone of this block moved
    /// directly to the right of another layout block.
    fn move_right_of_other(&self, renderer: &mut impl Renderer, other: &LayoutBlock) -> LayoutBlock {
        self.offset(other.area(renderer).width, 0)
    }

    /// Assuming that two layout blocks start at the same point, returns a clone of this block moved
    /// directly below another layout block.
    fn move_below_other(&self, renderer: &mut impl Renderer, other: &LayoutBlock) -> LayoutBlock {
        self.offset(0, other.area(renderer).height)
    }
}

pub trait Renderer {
    /// Given a glyph, returns the size that it will be drawn at. This is used to calculate the
    /// layout of the nodes before they are drawn.
    fn size(&mut self, glyph: Glyph) -> Area;

    /// Prepare a draw surface of the given size.
    fn init(&mut self, size: Area);

    /// Draw a glyph at a specific point.
    fn draw(&mut self, glyph: Glyph, point: CalculatedPoint);

    /// Computes the layout for a node tree, converting it into a set of glyphs at particular 
    /// locations.
    fn layout(&mut self, tree: &Node, path: Option<&mut NavPathNavigator>) -> LayoutBlock where Self: std::marker::Sized {        
        match tree {
            Node::Number(number) => {
                // We'll worry about negatives later!
                if *number < 0 { panic!("negative numbers not supported") }

                let glyph_layouts = (*number)
                    .to_string()
                    .chars()
                    .map(|c| Glyph::Digit { number: c.to_digit(10).unwrap() as u8 })
                    .map(|g| LayoutBlock::from_glyph(self, g))
                    .collect::<Vec<_>>();

                self.layout_horizontal(&glyph_layouts[..])
            },

            Node::Add(left, right) => self.layout_binop(Glyph::Add, left, right),
            Node::Subtract(left, right) => self.layout_binop(Glyph::Subtract, left, right),
            Node::Multiply(left, right) => self.layout_binop(Glyph::Multiply, left, right),

            Node::Divide(top, bottom) => {
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

                let top_layout = self.layout(
                    top, 
                    (&mut top_path).as_mut()
                );
                let bottom_layout = self.layout(
                    bottom,
                    (&mut bottom_path).as_mut()
                );

                // The fraction line should be the widest of the two
                let line_width = max(
                    top_layout.area(self).width,
                    bottom_layout.area(self).width,
                );
                let line_layout = LayoutBlock::from_glyph(self, Glyph::Fraction {
                    inner_width: line_width
                }).move_below_other(self, &top_layout);

                let bottom_layout = bottom_layout
                    .move_below_other(self, &line_layout);

                top_layout
                    .merge_along_vertical_centre(self, &line_layout, MergeBaseline::OtherAsBaseline)
                    .merge_along_vertical_centre(self, &bottom_layout, MergeBaseline::SelfAsBaseline)
            }

            Node::Token(token) => LayoutBlock::from_glyph(self, (*token).into()),

            Node::Unstructured(children) => {
                // We never actually mutate the paths...
                // Unsafe time!
                let mut paths = vec![];
                let mut cursorInsertionIndex = None;

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
                            cursorInsertionIndex = Some(p.as_mut().unwrap().next());
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
                    .map(|(i, node)| self.layout(
                        node,
                        (&mut paths[i]).as_mut()
                    ))
                    .collect::<Vec<_>>();

                // If the cursor is here, insert it
                if let Some(idx) = cursorInsertionIndex {
                    // TODO: height should be max of adjacents
                    layouts.insert(
                        idx, 
                        LayoutBlock::from_glyph(self, Glyph::Cursor {
                            height: 1,
                        })
                    )
                }

                self.layout_horizontal(&layouts[..])
            },

            _ => unimplemented!()
        }
    }

    /// Initialises the graphics surface and draws a node tree onto it.
    fn draw_all(&mut self, node: Node, path: Option<&mut NavPathNavigator>) where Self: std::marker::Sized {
        let layout = self.layout(&node, path);
        let area = layout.area(self);
        self.init(area);
        for (glyph, point) in layout.glyphs {
            self.draw(glyph, point);
        }
    }

    /// Returns the offset which should be applied to the y component of the `glyph` to vertically
    /// centre it in a container of height `height`.
    fn vertical_centre_glyph(&mut self, height: Dimension, glyph: Glyph) -> Dimension {
        (height - self.size(glyph).height) / 2
    }

    /// Calculates layout for a binop, with the operator being the `glyph`.
    fn layout_binop(&mut self, glyph: Glyph, left: &Node, right: &Node) -> LayoutBlock where Self: std::marker::Sized {
        // The navigation path is only for unstructured nodes, and the structured binops will never
        // appear will an unstructured tree (except divide, which is handled separately), so all
        // paths here are passed as None.

        let left_layout = self.layout(left, None);
        let binop_layout = LayoutBlock::from_glyph(self, glyph)
            .move_right_of_other(self, &left_layout);
        let right_layout = self.layout(right, None)
            .move_right_of_other(self, &binop_layout);

        left_layout
            .merge_along_baseline(&binop_layout)
            .merge_along_baseline(&right_layout)
    }

    /// Calculates layout for a sequence of other layouts, one-after-the-other horizontally.
    fn layout_horizontal(&mut self, layouts: &[LayoutBlock]) -> LayoutBlock
        where Self: std::marker::Sized
    {
        let mut block = LayoutBlock::empty();

        // Repeatedly merge the result block with a new block created to the right of it for
        // each glyph
        for layout in layouts {
            block = block.merge_along_baseline(
                &layout.move_right_of_other(self, &block),
            );
        }

        block
    }
}
