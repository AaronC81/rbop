use core::cmp::max;
use alloc::{vec::Vec, vec, string::ToString};
use crate::{StructuredNode, Token};

use crate::nav::NavPathNavigator;
use crate::node;

pub type Dimension = u64;

/// A point relative to the top-left of the layout.
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

    pub fn to_viewport_point(&self, viewport: Option<&Viewport>) -> ViewportPoint {
        if let Some(viewport) = viewport {
            ViewportPoint {
                x: self.x as i64 - viewport.offset.x as i64,
                y: self.y as i64 - viewport.offset.y as i64,
            }
        } else {
            ViewportPoint { x: self.x as i64, y: self.y as i64 }
        }
    }
}

pub struct Viewport {
    pub size: Area,
    pub offset: CalculatedPoint,
}

impl Viewport {
    pub fn new(size: Area) -> Self {
        Viewport { size, offset: CalculatedPoint { x: 0, y: 0 } }
    }

    pub fn includes_point(&self, point: &ViewportPoint) -> bool {
        // The ViewportPoint is relative to the top-left anyway, so the offset doesn't matter
        point.x >= 0 && point.y >= 0
        && point.x < self.size.width as i64 && point.y < self.size.height as i64
    }

    pub fn visibility(&self, point: &ViewportPoint, area: &Area) -> ViewportVisibility {
        let left_clip = if point.x < 0 { point.x.abs() } else { 0 } as u64;
        let top_clip = if point.y < 0 { point.y.abs() } else { 0 } as u64;

        let end_x = point.x + area.width as i64;
        let right_clip = if end_x > self.size.width as i64 {
            end_x - self.size.width as i64
        } else { 0 } as u64;

        let end_y = point.y + area.height as i64;
        let bottom_clip = if end_y > self.size.height as i64 {
            end_y - self.size.height as i64 
        } else { 0 } as u64;

        if top_clip == 0 && bottom_clip == 0 && left_clip == 0 && right_clip == 0 {
            ViewportVisibility::Visible
        } else {
            ViewportVisibility::Clipped { 
                invisible:  end_x < 0 || end_y < 0
                    || point.x > area.width as i64 || point.y > area.height as i64,
                top_clip, bottom_clip, left_clip, right_clip
            }
        }
    }
}

/// A point relative to the top-left of the viewport.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ViewportPoint {
    pub x: i64,
    pub y: i64,
}

impl ViewportPoint {
    pub fn dx(&self, delta: i64) -> ViewportPoint {
        ViewportPoint { x: self.x + delta, y: self.y }
    }

    pub fn dy(&self, delta: i64) -> ViewportPoint {
        ViewportPoint { x: self.x, y: self.y + delta }
    }
}

/// Describes the visibility of an item within a viewport.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ViewportVisibility {
    /// The entire item is visible.
    Visible,

    /// Some or all of the item is outside of the viewport.
    Clipped {
        /// True if the item is entirely invisible and does not need to be drawn at all.
        invisible: bool,

        /// The height from the top of the item which is clipped out of the viewport.
        top_clip: Dimension,
        
        /// The height from the bottom of the item which is clipped out of the viewport.
        bottom_clip: Dimension,

        /// The width from the left of the item which is clipped out of the viewport.
        left_clip: Dimension,

        /// The width from the right of the item which is clipped out of the viewport.
        right_clip: Dimension,
    },
}

/// A glyph in a viewport.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ViewportGlyph {
    pub glyph: Glyph,
    pub point: ViewportPoint,
    pub visibility: ViewportVisibility,
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
    Point,
    
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
            Token::Point => Glyph::Point,
        }
    }
}

impl Glyph {
    /// Returns the offset which should be applied to the y component of the `glyph` to vertically
    /// centre it in a container of height `height`.
    fn vertical_centre_glyph(&self, renderer: &mut impl Renderer, height: Dimension) -> Dimension {
        (height - renderer.size(*self).height) / 2
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
    pub fn empty() -> LayoutBlock {
        LayoutBlock { glyphs: vec![], baseline: 0 }
    }

    /// Creates a new layout block with one glyph at the origin. The baseline is the centre of this
    /// glyph.
    pub fn from_glyph(renderer: &mut impl Renderer, glyph: Glyph) -> LayoutBlock {
        LayoutBlock {
            glyphs: vec![(glyph, CalculatedPoint { x: 0, y: 0 })],
            baseline: renderer.size(glyph).height / 2,
        }
    }

    pub fn area(&self, renderer: &mut impl Renderer) -> Area {
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

    pub fn offset(&self, dx: Dimension, dy: Dimension) -> LayoutBlock {
        LayoutBlock {
            glyphs: self.glyphs
                .iter()
                .map(|(g, p)| (*g, p.dx(dx as i64).dy(dy as i64)))
                .collect(),
            baseline: self.baseline + dy,
        }
    }

    pub fn merge_along_baseline(&self, other: &LayoutBlock) -> LayoutBlock {
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
    pub fn merge_along_vertical_centre(&self, renderer: &mut impl Renderer, other: &LayoutBlock, baseline: MergeBaseline) -> LayoutBlock {
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

    /// Merge the the glyphs of two layout blocks exactly, without moving them.
    pub fn merge_in_place(&self, renderer: &mut impl Renderer, other: &LayoutBlock, baseline: MergeBaseline) -> LayoutBlock {
        let glyphs =
            // Re-align the lesser-baselined glyphs
            self.glyphs
            .iter()
            .cloned()
            // Chain with the unmodified greater-baselined glyphs
            .chain(other.glyphs.iter().cloned())
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
    pub fn move_right_of_other(&self, renderer: &mut impl Renderer, other: &LayoutBlock) -> LayoutBlock {
        self.offset(other.area(renderer).width, 0)
    }

    /// Assuming that two layout blocks start at the same point, returns a clone of this block moved
    /// directly below another layout block.
    pub fn move_below_other(&self, renderer: &mut impl Renderer, other: &LayoutBlock) -> LayoutBlock {
        self.offset(0, other.area(renderer).height)
    }

    /// Calculates layout for a sequence of other layouts, one-after-the-other horizontally.
    pub fn layout_horizontal(renderer: &mut impl Renderer, layouts: &[LayoutBlock]) -> LayoutBlock where Self: Sized
    {
        let mut block = LayoutBlock::empty();

        // Repeatedly merge the result block with a new block created to the right of it for
        // each glyph
        for layout in layouts {
            block = block.merge_along_baseline(
                &layout.move_right_of_other(renderer, &block),
            );
        }

        block
    }

    pub fn for_viewport(&self, renderer: &mut impl Renderer, viewport: Option<&Viewport>) -> Vec<ViewportGlyph> {
        self.glyphs
            .iter()
            .map(|(g, p)| {
                let viewport_point = p.to_viewport_point(viewport);
                ViewportGlyph {
                    glyph: *g,
                    point: viewport_point,
                    visibility: if let Some(viewport) = viewport {
                        viewport.visibility(&viewport_point, &renderer.size(*g))
                    } else {
                        ViewportVisibility::Visible
                    }, 
                }
            })
            .collect::<Vec<_>>()
    } 
}

pub trait Layoutable {
    /// Computes the layout for a node tree, converting it into a set of glyphs at particular 
    /// locations.
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>) -> LayoutBlock;
}

pub trait Renderer {
    /// Given a glyph, returns the size that it will be drawn at. This is used to calculate the
    /// layout of the nodes before they are drawn.
    fn size(&mut self, glyph: Glyph) -> Area;

    /// Prepare a draw surface of the given size.
    fn init(&mut self, size: Area);

    /// Draw a glyph at a specific point.
    fn draw(&mut self, glyph: ViewportGlyph);

    /// Computes the layout for a node tree, converting it into a set of glyphs at particular 
    /// locations.
    fn layout(&mut self, root: &impl Layoutable, path: Option<&mut NavPathNavigator>) -> LayoutBlock where Self: Sized {
        root.layout(self, path)
    }

    /// Initialises the graphics surface and draws a node tree onto it.
    fn draw_all(&mut self, root: &impl Layoutable, path: Option<&mut NavPathNavigator>, viewport: Option<&Viewport>) where Self: Sized {
        let layout = self.layout(root, path); 
        let area = if let Some(v) = viewport {
            v.size
        } else {
            layout.area(self)
        };

        let viewport_glyphs = layout.for_viewport(self, viewport);

        self.init(area);
        for glyph in viewport_glyphs {
            self.draw(glyph);
        }
    }

    /// Returns the visibility of the cursor when rendering a set of nodes in a viewport.
    fn cursor_visibility(&mut self, root: &impl Layoutable, path: &mut NavPathNavigator, viewport: Option<&Viewport>) -> ViewportVisibility where Self: Sized {
        let layout = self.layout(root, Some(path)); 
        let viewport_glyphs = layout.for_viewport(self, viewport);

        for glyph in viewport_glyphs {
            if let ViewportGlyph { glyph: Glyph::Cursor { .. }, visibility, .. } = glyph {
                return visibility
            }
        }

        panic!("cursor was not rendered");
    }
}
