#![feature(box_syntax)]
#![feature(result_flattening)]

use std::rc::Rc;

use speedy2d::{self, Graphics2D, Window, color::Color, font::{Font, FormattedTextBlock, TextLayout, TextOptions}, window::{VirtualKeyCode, WindowHandler, WindowHelper}};
use rbop::{Token, UnstructuredNode, UnstructuredNodeList, node::unstructured::{UnstructuredNodeRoot, Upgradable}, render::Renderer};

struct Speedy2DRenderer<'a> {
    graphics: Option<&'a mut Graphics2D>,
    font: Font,
}

impl<'a> Speedy2DRenderer<'a> {
    fn text_layout<T>(&mut self, text: T) -> Rc<FormattedTextBlock> where T : Into<String> {
        self.font.layout_text(&text.into(), 50.0, TextOptions::new())
    }

    fn text_size<T>(&mut self, text: T) -> rbop::render::Area where T : Into<String> {
        let layout = self.text_layout(text);
        rbop::render::Area {
            width: layout.width() as u64,
            height: layout.height() as u64,
        }
    }

    fn text_draw<T>(&mut self, text: T, point: rbop::render::CalculatedPoint) where T : Into<String> {
        let layout = &self.text_layout(text.into());
        self.graphics.as_mut().unwrap().draw_text(
            (point.x as f32, point.y as f32),
            Color::BLACK,
            layout
        );
    }
}

impl<'a> Renderer for Speedy2DRenderer<'a> {
    fn size(&mut self, glyph: rbop::render::Glyph) -> rbop::render::Area {
        match glyph {
            rbop::render::Glyph::Digit { number } => 
                self.text_size(format!("{}", number)),

            rbop::render::Glyph::Add => self.text_size("+"),
            rbop::render::Glyph::Subtract => self.text_size("-"),
            rbop::render::Glyph::Multiply => self.text_size("*"),
            rbop::render::Glyph::Divide => self.text_size("/"),

            rbop::render::Glyph::Fraction { inner_width } => rbop::render::Area {
                width: inner_width,
                height: 3,
            },

            rbop::render::Glyph::LeftParenthesis { inner_height } => todo!(),
            rbop::render::Glyph::RightParenthesis { inner_height } => todo!(),
            rbop::render::Glyph::Sqrt { inner_area } => todo!(),

            rbop::render::Glyph::Cursor { height } => rbop::render::Area {
                // Lie about the cursor width! This means that rbop doesn't make space for the
                // cursor, so the cursor moving won't cause elements to shift a little bit
                width: 0,
                height,
            },
        }
    }

    fn init(&mut self, size: rbop::render::Area) {
        self.graphics.as_mut().unwrap().clear_screen(Color::from_rgb(1.0, 1.0, 1.0));
    }

    fn draw(&mut self, glyph: rbop::render::Glyph, point: rbop::render::CalculatedPoint) {
        // Offset a little bit from the origin
        let point = point.dx(20).dy(20);

        match glyph {
            rbop::render::Glyph::Digit { number } =>
                self.text_draw(format!("{}", number), point),
            rbop::render::Glyph::Add => self.text_draw("+", point),
            rbop::render::Glyph::Subtract => self.text_draw("-", point),
            rbop::render::Glyph::Multiply => self.text_draw("*", point),
            rbop::render::Glyph::Divide => self.text_draw("/", point),

            rbop::render::Glyph::Fraction { inner_width } => 
                self.graphics.as_mut().unwrap().draw_line(
                    (point.x as f32, point.y as f32),
                    (point.x as f32 + inner_width as f32, point.y as f32),
                    3.0,
                    Color::BLACK
                ),

            rbop::render::Glyph::LeftParenthesis { inner_height } => todo!(),
            rbop::render::Glyph::RightParenthesis { inner_height } => todo!(),
            rbop::render::Glyph::Sqrt { inner_area } => todo!(),

            rbop::render::Glyph::Cursor { height } =>
                self.graphics.as_mut().unwrap().draw_line(
                    (point.x as f32, point.y as f32),
                    (point.x as f32, point.y as f32 + height as f32),
                    1.0,
                    Color::BLACK
                ),
        }
    }
}

struct WindowCalc {
    node: UnstructuredNodeRoot,
    nav_path: rbop::nav::NavPath,
    needs_draw: bool,
}

impl WindowCalc {
    fn new_window() -> Window {
        Window::new_centered("Window Calc", (640, 480))
            .expect("unable to create window")
    }

    fn create_renderer<'a>(&mut self, graphics: Option<&'a mut Graphics2D>) -> Speedy2DRenderer<'a> {
        let font = include_bytes!("/usr/share/fonts/RobotoSlab-Regular.ttf");
        Speedy2DRenderer {
            graphics,
            font: Font::new(font).unwrap(),
        }
    }
}

impl WindowHandler for WindowCalc {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        if self.needs_draw {
            graphics.clear_screen(Color::from_rgb(1.0, 1.0, 1.0));

            // A bit funky, but makes the borrow checker happy
            // Means that we drop create_renderer's mutable borrow before using `draw_text` again
            let result_text = {
                let mut renderer = self.create_renderer(Some(graphics));
                renderer.draw_all(&self.node, Some(&mut self.nav_path.to_navigator()));

                let result = self.node.upgrade().map(|x| x.evaluate()).flatten();
                renderer.text_layout(match result {
                    Ok(number) => format!("{}", number),
                    Err(error) => error.to_string(),   
                })
            };

            graphics.draw_text(
                (20.0, 400.0),
                Color::BLACK,
                &result_text,
            );

            self.needs_draw = false;
        }

        helper.request_redraw();
    }

    fn on_key_down(&mut self, helper: &mut WindowHelper<()>, virtual_key_code: Option<VirtualKeyCode>, scancode: speedy2d::window::KeyScancode) {
        match virtual_key_code.unwrap() {
            VirtualKeyCode::Key0 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(0))),
            VirtualKeyCode::Key1 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(1))),
            VirtualKeyCode::Key2 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(2))),
            VirtualKeyCode::Key3 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(3))),
            VirtualKeyCode::Key4 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(4))),
            VirtualKeyCode::Key5 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(5))),
            VirtualKeyCode::Key6 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(6))),
            VirtualKeyCode::Key7 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(7))),
            VirtualKeyCode::Key8 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(8))),
            VirtualKeyCode::Key9 => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Digit(9))),

            VirtualKeyCode::Plus => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Add)),
            VirtualKeyCode::Minus => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Subtract)),
            VirtualKeyCode::Asterisk => self.node.insert(&mut self.nav_path, UnstructuredNode::Token(Token::Multiply)),
            VirtualKeyCode::Slash => self.node.insert(&mut self.nav_path, UnstructuredNode::Fraction(
                UnstructuredNodeList { items: vec![] },
                UnstructuredNodeList { items: vec![] },
            )),

            VirtualKeyCode::S => self.node.insert(&mut self.nav_path, UnstructuredNode::Sqrt(
                UnstructuredNodeList { items: vec![] },
            )),

            VirtualKeyCode::Left => self.node.move_left(&mut self.nav_path),
            VirtualKeyCode::Right => self.node.move_right(&mut self.nav_path),
            VirtualKeyCode::Down => {
                let renderer = &mut self.create_renderer(None);
                self.node.move_down(&mut self.nav_path, renderer)
            },
            VirtualKeyCode::Up => {
                let renderer = &mut self.create_renderer(None);
                self.node.move_up(&mut self.nav_path, renderer)
            },

            VirtualKeyCode::Backspace => self.node.delete(&mut self.nav_path),
            _ => (),
        }

        self.needs_draw = true;
    }
}

fn main() {
    WindowCalc::new_window().run_loop(WindowCalc {
        node: UnstructuredNodeRoot { root: UnstructuredNodeList { items: vec![] } },
        nav_path: rbop::nav::NavPath::new(vec![0]),
        needs_draw: true,
    })
}
