use std::{alloc::Layout, cmp::max, unimplemented};

use crate::Node;

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
    x: u64,
    y: u64,
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
    width: Dimension,
    height: Dimension,
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

    Fraction { inner_width: Dimension },

    LeftParenthesis { inner_height: Dimension },
    RightParenthesis { inner_height: Dimension },

    Sqrt { inner_area: Area },
}

pub struct LayoutBlock {
    // TODO: we probably do still need calculatingpoint for aligning common baselines
    // e.g. 
    //      .-
    // 1   \|2
    // - + ---
    // 2    5
    pub glyphs: Vec<(Glyph, CalculatedPoint)>,
}

impl LayoutBlock {
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
                .collect()
        }
    }

    fn merge(&self, other: LayoutBlock) -> LayoutBlock {
        LayoutBlock {
            glyphs: self.glyphs.iter().cloned().chain(other.glyphs).collect()
        }
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
    fn layout(&mut self, tree: &Node) -> LayoutBlock where Self: std::marker::Sized {        
        match tree {
            Node::Number(number) => {
                // We'll worry about negatives later!
                if *number < 0 { panic!("negative numbers not supported") }

                let glyphs = (*number)
                    .to_string()
                    .chars()
                    .map(|c| Glyph::Digit { number: c.to_digit(10).unwrap() as u8 })
                    .collect::<Vec<_>>();

                let max_height = glyphs.iter().map(|g| self.size(*g).height).max().unwrap();

                let mut glyphs_with_points = vec![];
                let mut current_x = 0;

                for glyph in glyphs {
                    glyphs_with_points.push((glyph, CalculatedPoint {
                        x: current_x,
                        y: self.vertical_centre_glyph(max_height, glyph),
                    }));

                    let size = self.size(glyph);
                    current_x += size.width;
                }

                LayoutBlock { glyphs: glyphs_with_points }
            },

            Node::Add(left, right) => self.layout_binop(Glyph::Add, left, right),
            Node::Subtract(left, right) => self.layout_binop(Glyph::Subtract, left, right),
            Node::Multiply(left, right) => self.layout_binop(Glyph::Multiply, left, right),

            Node::Token(_) | Node::Unstructured(_) => panic!("must upgrade to render"),

            _ => unimplemented!()
        }
    }

    /// Initialises the graphics surface and draws a node tree onto it.
    fn draw_all(&mut self, node: Node) where Self: std::marker::Sized {
        let layout = self.layout(&node);
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
        let left_layout = self.layout(left);
        let right_layout = self.layout(right);

        let max_height = max(
            left_layout.area(self).height, 
            right_layout.area(self).height
        );

        let x_offset_after_left = left_layout.area(self).width;

        let binop_glyph_layout = LayoutBlock {
            glyphs: vec![
                (glyph, CalculatedPoint {
                    x: x_offset_after_left,
                    // TODO: should align to baseline rather than vertical centre
                    y: self.vertical_centre_glyph(max_height, Glyph::Add),
                })
            ]
        };

        let x_offset_after_binop = x_offset_after_left + self.size(Glyph::Add).width;

        // TODO: ditto re v.centre
        left_layout.offset(0, (max_height - left_layout.area(self).height) / 2)
            .merge(binop_glyph_layout)
            .merge(right_layout.offset(x_offset_after_binop, (max_height - right_layout.area(self).height) / 2))
    }
}

#[derive(Default, Clone, Debug)]
pub struct AsciiRenderer {
    pub lines: Vec<String>,
}

impl AsciiRenderer {
    fn put_char(&mut self, char: char, point: CalculatedPoint) {
        self.lines[point.y as usize].replace_range(
            (point.x as usize)..(point.x as usize + 1),
            &char.to_string()
        );
    }
}

impl Renderer for AsciiRenderer {
    fn size(&mut self, glyph: Glyph) -> Area {
        match glyph {
            Glyph::Digit { .. } | Glyph::Add | Glyph::Subtract | Glyph::Multiply => Area::square(1),

            Glyph::Fraction { inner_width } => Area::new(inner_width, 1),

            // TODO: currently we'll just force the inner area into the bottom right, we may want to
            // offer more granular control of this in future
            Glyph::Sqrt { inner_area } => Area::new(inner_area.width + 2, inner_area.height + 1),

            Glyph::LeftParenthesis { inner_height } | Glyph::RightParenthesis { inner_height }
                => Area::new(1, inner_height),
        }
    }

    fn init(&mut self, size: Area) {
        self.lines = Vec::new();
        for _ in 0..size.height {
            self.lines.push(" ".repeat(size.width as usize));
        }
    }

    fn draw(&mut self, glyph: Glyph, point: CalculatedPoint) {
        match glyph {
            Glyph::Digit { number } => {
                let char = number.to_string().chars().next().unwrap();
                self.put_char(char, point);
            },
            Glyph::Add => self.put_char('+', point),
            Glyph::Subtract => self.put_char('-', point),
            Glyph::Multiply => self.put_char('*', point),
            Glyph::Fraction { inner_width } => {
                for dx in 0..inner_width {
                    self.put_char('-', point.dx(dx as i64))
                }
            },
            Glyph::LeftParenthesis { inner_height } => {
                if inner_height == 1 {
                    self.put_char('(', point)
                } else {
                    self.put_char('/', point);
                    for dy in 1..(inner_height - 1) {
                        self.put_char('|', point.dy(dy as i64))
                    }
                    self.put_char('\\', point.dy(inner_height as i64));
                }
            },
            Glyph::RightParenthesis { inner_height } => {
                if inner_height == 1 {
                    self.put_char(')', point)
                } else {
                    self.put_char('\\', point);
                    for dy in 1..(inner_height - 1) {
                        self.put_char('|', point.dy(dy as i64));
                    }
                    self.put_char('/', point.dy(inner_height as i64));
                }
            },
            Glyph::Sqrt { inner_area } => {
                self.put_char('\\', CalculatedPoint {
                    x: point.x,
                    y: point.y + inner_area.height,
                });
                for dy in 1..inner_area.height {
                    self.put_char('|', point.dx(1).dy(dy as i64));
                }
                self.put_char('.', point.dx(1));
                for dx in 2..inner_area.width {
                    self.put_char('-', point.dx(dx as i64));
                }
            }
        }
    }
}
