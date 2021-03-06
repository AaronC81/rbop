use alloc::{vec::Vec, vec};
use crate::Token;

use crate::nav::NavPathNavigator;
use crate::node::function::Function;

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
                invisible: end_x + (area.width as i64) < 0 || end_y + (area.height as i64) < 0
                    || point.x > self.size.width as i64 || point.y > self.size.height as i64,
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
    pub glyph: SizedGlyph,
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

    FunctionName { function: Function },
    Comma,

    Variable { name: char },
    
    Add,
    Subtract,
    Multiply,
    Divide,

    Fraction { inner_width: Dimension },

    LeftParenthesis { inner_height: Dimension },
    RightParenthesis { inner_height: Dimension },

    Sqrt { inner_area: Area },

    Cursor { height: Dimension },
    Placeholder,
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
            Token::Variable(c) => Glyph::Variable { name: c },
        }
    }
}

impl Glyph {
    pub fn to_sized(self, renderer: &mut impl Renderer, size_reduction_level: u32) -> SizedGlyph {
        SizedGlyph::from_glyph(self, renderer, size_reduction_level)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct SizedGlyph {
    pub glyph: Glyph,
    pub area: Area,
    pub size_reduction_level: u32,
}

impl SizedGlyph {
    pub fn from_glyph(glyph: Glyph, renderer: &mut impl Renderer, size_reduction_level: u32) -> Self {
        SizedGlyph {
            glyph,
            area: renderer.size(glyph, size_reduction_level),
            size_reduction_level,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LayoutBlock {
    pub glyphs: Vec<(SizedGlyph, CalculatedPoint)>,
    pub baseline: Dimension,
    pub area: Area,
    pub special: LayoutBlockSpecial,
}


/// A set of rarely-used special flags which a layout block may use to control unusual behaviours
/// while computing a layout.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct LayoutBlockSpecial {
    /// If true, while merging along the horizontal baseline with `layout_horizontal`, this node
    /// will be merged with its left peer before any other merges take place.
    ///
    /// If multiple layout blocks specify this option to have high precedence, they will be merged
    /// from lowest index (left) to highest index (right).
    ///
    /// This can be used to ensure that blocks which should appear as a superscript appear at the
    /// correct height. For example, let's render (1/2) + 3^2:
    ///
    /// ```text
    ///    Normal prec.                    High prec.
    /// 
    ///     .-------.2 <-----.          .-----. .----. 
    ///     | 1     |        |          | 1   | |  2 |
    ///    ~| - + 3 |~       |         ~| - + |~| 3  |~
    ///     | 2     |        |          | 2   | '----' 
    ///     '-------'        |          '-----'  ^^^^
    ///      ^^^^^^^         |                   Merged
    ///   Left blocks were   |                   first 
    ///   merged first...    |                    
    ///     so power is -----'                    
    ///    wrong height     
    /// ```               
    ///
    pub baseline_merge_with_high_precedence: bool,

    /// If true, while merging along the horizontal baseline with `merge_along_baseline`, this block
    /// will be placed vertically above the block before it. The horizontal placement will not be
    /// affected.
    ///
    /// It is very likely you also want `baseline_merge_with_high_precedence` to be true when using
    /// this option.
    pub superscript: bool,
}

pub enum MergeBaseline {
    SelfAsBaseline,
    OtherAsBaseline,
}

impl LayoutBlock {
    pub fn empty() -> LayoutBlock {
        LayoutBlock { glyphs: vec![], baseline: 0, area: Area::new(0, 0), special: LayoutBlockSpecial::default() }
    }

    pub fn new(glyphs: Vec<(SizedGlyph, CalculatedPoint)>, baseline: Dimension) -> Self {
        let area =  Self::area(&glyphs);
        Self {
            glyphs,
            baseline,
            area,
            special: LayoutBlockSpecial::default(),
        }
    }

    pub fn update_area(self) -> Self {
        Self {
            area: Self::area(&self.glyphs),
            ..self
        }
    }

    /// Creates a new layout block with one glyph at the origin. The baseline is the centre of this
    /// glyph.
    pub fn from_glyph(renderer: &mut impl Renderer, glyph: Glyph, properties: LayoutComputationProperties) -> LayoutBlock {
        let glyph = glyph.to_sized(renderer, properties.size_reduction_level);
        LayoutBlock {
            glyphs: vec![(glyph, CalculatedPoint { x: 0, y: 0 })],
            baseline: glyph.area.height / 2,
            area: glyph.area,
            special: LayoutBlockSpecial::default(),
        }
    }

    fn area(glyphs: &[(SizedGlyph, CalculatedPoint)]) -> Area {
        let mut width = 0;
        let mut height = 0;

        for (glyph, point) in glyphs {
            let size = glyph.area;
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
            ..*self
        }.update_area()
    }

    pub fn merge_along_baseline(&self, other: &LayoutBlock) -> LayoutBlock {
        // Is the other block is a superscript? This needs to be handled totally differently...
        if other.special.superscript {
            // Currently the superscript special is only used by powers, so I'm just going to call
            // the normally-aligned part (&self) the base, and the superscript-aligned part (other)
            // the exponent!
            
            // Shift this block, the base, down by the exponent's height
            // Ideally we'd shift the exponent up, but we can't shift negatively - we can use this
            // to achieve the same thing though
            let base = self.offset(0, other.area.height);

            // There's no need to shift the exponent - we expect that it was moved to the right
            // prior to calling this

            // Merge them both in place! Use the base's baseline
            return base.merge_in_place(other, MergeBaseline::SelfAsBaseline);

            // TODO: cursor plus superscript is currently quite broken:
            //   - It'll bind to the cursor if it's before one
            //   - The cursor isn't the correct height after one (which would actually solve the
            //     first issue, I think?)
        }

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

        LayoutBlock::new(glyphs, greater_baselined.baseline)
    }

    /// Merges the glyphs of two layout blocks along their vertical centre.
    pub fn merge_along_vertical_centre(&self, other: &LayoutBlock, baseline: MergeBaseline) -> LayoutBlock {
        // Whose is wider? (i.e., who has the greatest vertical centre)
        // The points can't go negative, so we'll add to the glyphs of the smaller layout block
        let self_centre = self.area.width / 2;
        let other_centre = other.area.width / 2;
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

        LayoutBlock::new(glyphs, match baseline {
            MergeBaseline::SelfAsBaseline => self.baseline,
            MergeBaseline::OtherAsBaseline => other.baseline,
        })
    }

    /// Merge the the glyphs of two layout blocks exactly, without moving them.
    pub fn merge_in_place(&self, other: &LayoutBlock, baseline: MergeBaseline) -> LayoutBlock {
        let glyphs =
            // Re-align the lesser-baselined glyphs
            self.glyphs
            .iter()
            .cloned()
            // Chain with the unmodified greater-baselined glyphs
            .chain(other.glyphs.iter().cloned())
            .collect::<Vec<_>>();

        LayoutBlock::new(glyphs, match baseline {
            MergeBaseline::SelfAsBaseline => self.baseline,
            MergeBaseline::OtherAsBaseline => other.baseline,
        })
    }

    /// Assuming that two layout blocks start at the same point, returns a clone of this block moved
    /// directly to the right of another layout block.
    pub fn move_right_of_other(&self, other: &LayoutBlock) -> LayoutBlock {
        self.offset(other.area.width, 0)
    }

    /// Assuming that two layout blocks start at the same point, returns a clone of this block moved
    /// directly below another layout block.
    pub fn move_below_other(&self, other: &LayoutBlock) -> LayoutBlock {
        self.offset(0, other.area.height)
    }

    /// Calculates layout for a sequence of other layouts, one-after-the-other horizontally.
    pub fn layout_horizontal(layouts: &[LayoutBlock]) -> LayoutBlock where Self: Sized
    {
        let mut block = LayoutBlock::empty();

        // Check for layouts which want to be merged with high precedence
        let mut layouts = layouts.to_vec();

        // Actual indexes will change as we're iterating - store an offset
        let mut i_offset = 0;
        for (i, layout) in layouts.clone().into_iter().enumerate() {
            let i = i - i_offset;
            if layout.special.baseline_merge_with_high_precedence {
                // Get the layout to this one's left (or do nothing if it's at the start)
                if i > 0 {
                    let layout_to_left = layouts[i - 1].clone();

                    // Remove both layouts
                    layouts.drain((i - 1)..=i);

                    // Insert new merged layout
                    layouts.insert(i - 1, layout_to_left.merge_along_baseline(
                        &layout.move_right_of_other(&layout_to_left)
                    ));

                    // Indexes of future items have shifted down by 1
                    i_offset += 1;
                }
            }
        }

        // Repeatedly merge the result block with a new block created to the right of it for
        // each glyph
        for layout in layouts {
            block = block.merge_along_baseline(
                &layout.move_right_of_other(&block),
            );
        }

        block
    }

    pub fn for_viewport(&self, viewport: Option<&Viewport>) -> Vec<ViewportGlyph> {
        self.glyphs
            .iter()
            .map(|(g, p)| {
                let viewport_point = p.to_viewport_point(viewport);
                ViewportGlyph {
                    glyph: *g,
                    point: viewport_point,
                    visibility: if let Some(viewport) = viewport {
                        viewport.visibility(&viewport_point, &g.area)
                    } else {
                        ViewportVisibility::Visible
                    }, 
                }
            })
            .collect::<Vec<_>>()
    } 
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct LayoutComputationProperties {
    pub size_reduction_level: u32
}

impl Default for LayoutComputationProperties {
    fn default() -> Self {
        LayoutComputationProperties {
            size_reduction_level: 0,
        }
    }
}

impl LayoutComputationProperties {
    pub fn reduce_size(self) -> Self {
        Self { size_reduction_level: self.size_reduction_level + 1, ..self }
    }
}

pub trait Layoutable {
    /// Computes the layout for a node tree, converting it into a set of glyphs at particular 
    /// locations.
    fn layout(&self, renderer: &mut impl Renderer, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> LayoutBlock;
}

pub trait Renderer {
    /// Given a glyph, returns the size that it will be drawn at. This is used to calculate the
    /// layout of the nodes before they are drawn.
    fn size(&mut self, glyph: Glyph, size_reduction_level: u32) -> Area;

    /// Prepare a draw surface of the given size.
    fn init(&mut self, size: Area);

    /// Draw a glyph at a specific point.
    fn draw(&mut self, glyph: ViewportGlyph);

    /// Computes the layout for a node tree, converting it into a set of glyphs at particular 
    /// locations.
    fn layout(&mut self, root: &impl Layoutable, path: Option<&mut NavPathNavigator>, properties: LayoutComputationProperties) -> LayoutBlock where Self: Sized {
        root.layout(self, path, properties)
    }

    /// Initialises the graphics surface and draws a node tree onto it.
    fn draw_all(&mut self, root: &impl Layoutable, path: Option<&mut NavPathNavigator>, viewport: Option<&Viewport>) -> LayoutBlock where Self: Sized {
        let layout = self.layout(root, path, LayoutComputationProperties::default()); 
        self.draw_all_by_layout(&layout, viewport);
        layout
    }

    /// Initializes the graphics surface and draws a node tree onto it, assuming that a layout has
    /// already been calculated.
    fn draw_all_by_layout(&mut self, layout: &LayoutBlock, viewport: Option<&Viewport>) where Self: Sized {
        let area = if let Some(v) = viewport {
            v.size
        } else {
            layout.area
        };

        let viewport_glyphs = layout.for_viewport(viewport);

        self.init(area);
        for glyph in viewport_glyphs {
            self.draw(glyph);
        }
    }

    /// Returns the visibility of the cursor when rendering a set of nodes in a viewport.
    fn cursor_visibility(&mut self, root: &impl Layoutable, path: &mut NavPathNavigator, viewport: Option<&Viewport>) -> ViewportVisibility where Self: Sized {
        let layout = self.layout(root, Some(path), LayoutComputationProperties::default()); 
        let viewport_glyphs = layout.for_viewport(viewport);

        for glyph in viewport_glyphs {
            if let ViewportGlyph { glyph: SizedGlyph { glyph: Glyph::Cursor { .. }, .. }, visibility, .. } = glyph {
                return visibility
            }
        }

        panic!("cursor was not rendered");
    }

    /// An overridable special option: the padding from the right of a square root node where the
    /// inner expression should be rendered. 
    fn square_root_padding(&self) -> u64 { 0 }
}
