use crate::render::{Area, CalculatedPoint, Glyph, Renderer, ViewportGlyph, ViewportPoint, ViewportVisibility};
use alloc::{vec::Vec, string::{String, ToString}};

#[derive(Default, Clone, Debug)]
pub struct AsciiRenderer {
    pub lines: Vec<String>,
}

impl AsciiRenderer {
    fn put_char(&mut self, char: char, point: ViewportPoint) {
        self.lines[point.y as usize].replace_range(
            (point.x as usize)..(point.x as usize + 1),
            &char.to_string()
        );
    }
}

impl Renderer for AsciiRenderer {
    fn size(&mut self, glyph: Glyph) -> Area {
        match glyph {
            Glyph::Digit { .. } | Glyph::Point | Glyph::Add | Glyph::Subtract | Glyph::Multiply | Glyph::Divide => Area::square(1),

            Glyph::Fraction { inner_width } => Area::new(inner_width, 1),

            // TODO: currently we'll just force the inner area into the bottom right, we may want to
            // offer more granular control of this in future
            Glyph::Sqrt { inner_area } => Area::new(inner_area.width + 2, inner_area.height + 1),

            Glyph::LeftParenthesis { inner_height } | Glyph::RightParenthesis { inner_height }
                => Area::new(1, inner_height),

            Glyph::Cursor { height } => Area::new(1, height),
        }
    }

    fn init(&mut self, size: Area) {
        self.lines = Vec::new();
        for _ in 0..size.height {
            self.lines.push(" ".repeat(size.width as usize));
        }
    }

    fn draw(&mut self, mut viewport_glyph: ViewportGlyph) {
        match viewport_glyph.visibility {
            ViewportVisibility::Visible => (),
            ViewportVisibility::Clipped { invisible, .. } if invisible => return,
            ViewportVisibility::Clipped { left_clip, right_clip, .. } => {
                // TODO: support other glyphs clipped
                let mut preserve_this_glyph = false;

                // Re-align and shorten a left-clipped fraction line
                if let Glyph::Fraction { inner_width } = viewport_glyph.glyph {
                    if left_clip > 0 {
                        viewport_glyph.glyph = Glyph::Fraction {
                            inner_width: inner_width - left_clip
                        };
                        viewport_glyph.point.x = 0;

                        preserve_this_glyph = true;
                    }
                }

                // Shorten a right-clipped fraction line
                // (The if-let binding is repeated to get a possibly updated inner_width)
                if let Glyph::Fraction { inner_width } = viewport_glyph.glyph {
                    if right_clip > 0 {
                        viewport_glyph.glyph = Glyph::Fraction {
                            inner_width: inner_width - right_clip
                        };

                        preserve_this_glyph = true;
                    }
                }

                // We weren't able to handle the clip, just forget this glyph
                if !preserve_this_glyph {
                    return;
                }
            }
        } 

        let point = viewport_glyph.point;

        match viewport_glyph.glyph {
            Glyph::Digit { number } => {
                let char = number.to_string().chars().next().unwrap();
                self.put_char(char, point);
            },
            Glyph::Point => self.put_char('.', point),
            Glyph::Add => self.put_char('+', point),
            Glyph::Subtract => self.put_char('-', point),
            Glyph::Multiply => self.put_char('*', point),
            Glyph::Divide => self.put_char('/', point),
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
                    self.put_char('\\', point.dy(inner_height as i64 - 1));
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
                    self.put_char('/', point.dy(inner_height as i64 - 1));
                }
            },
            Glyph::Sqrt { inner_area } => {
                self.put_char('\\', ViewportPoint {
                    x: point.x,
                    y: point.y + inner_area.height as i64,
                });
                for dy in 1..=inner_area.height {
                    self.put_char('|', point.dx(1).dy(dy as i64));
                }
                self.put_char('.', point.dx(1));
                for dx in 2..(2+inner_area.width) {
                    self.put_char('-', point.dx(dx as i64));
                }
            },
            Glyph::Cursor { height } => {
                for dy in 0..height {
                    self.put_char('|', point.dy(dy as i64))
                }
            },
        }
    }
}
