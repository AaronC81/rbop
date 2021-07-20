use crate::render::{Renderer, CalculatedPoint, Area, Glyph};

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
            Glyph::Digit { .. } | Glyph::Add | Glyph::Subtract | Glyph::Multiply | Glyph::Divide => Area::square(1),

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

    fn draw(&mut self, glyph: Glyph, point: CalculatedPoint) {
        match glyph {
            Glyph::Digit { number } => {
                let char = number.to_string().chars().next().unwrap();
                self.put_char(char, point);
            },
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
