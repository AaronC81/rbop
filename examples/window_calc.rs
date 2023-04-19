// This example shows pretty much a complete rbop usage. It implements its own renderer for drawing
// onto a Speedy2D canvas, accepts input, and evaluates the result.
//
// If you are reading these examples to become familiar with rbop, it is recommended that you read
// the `ascii_calc` example first, as that will contain a more thorough description of handling
// input and evaluation. This will focus more on the implementation of the renderer.

#![feature(result_flattening)]

// Enforce `examples` feature is passed ------------------------------------------------------------
#[cfg(not(feature = "examples"))]
mod window_calc {
    pub const CHECK: usize = panic!("you must enable the `examples` feature to compile examples.");
}

#[cfg(not(feature = "examples"))]
fn main() {
    println!("{}", window_calc::CHECK);
}
// -------------------------------------------------------------------------------------------------

#[cfg(feature = "examples")]
mod window_calc {
    use std::rc::Rc;
    use rbop::{Token, UnstructuredNode, UnstructuredNodeList, node::{unstructured::{UnstructuredNodeRoot, Upgradable}, structured::EvaluationSettings}, render::{Renderer, SizedGlyph, ViewportGlyph}};
    use speedy2d::{self, Graphics2D, Window, color::Color, font::{Font, FormattedTextBlock, TextLayout, TextOptions}, window::{VirtualKeyCode, WindowHandler, WindowHelper}};

    // This is the struct we'll implement `Renderer` on! The fields will be very
    // implementation-specific, here we're holding a reference to the Speedy2D graphics surface and
    // font.
    struct Speedy2DRenderer<'a> {
        graphics: Option<&'a mut Graphics2D>,
        font: Font,
    }

    impl<'a> Speedy2DRenderer<'a> {
        /// Uses this renderer's font to lay out the given `text` into a `FormattedTextBlock`.
        fn text_layout(&mut self, text: &str, size_reduction_level: u32) -> Rc<FormattedTextBlock> {
            // Half the default text size for each level of size reduction
            let mut size = 50.0;
            for _ in 0..size_reduction_level {
                size /= 2.0;
            }
            self.font.layout_text(text, size, TextOptions::new())
        }

        /// Returns the size of the given `text when rendered using this renderer's font.
        fn text_size(&mut self, text: &str, size_reduction_level: u32) -> rbop::render::Area {
            let layout = self.text_layout(text, size_reduction_level);
            rbop::render::Area {
                width: layout.width() as u64,
                height: layout.height() as u64,
            }
        }

        /// Draws `text` onto the graphics surface at `point`, using this renderer's font.
        fn text_draw(&mut self, text: &str, point: rbop::render::ViewportPoint, size_reduction_level: u32) {
            let layout = &self.text_layout(text, size_reduction_level);
            self.graphics.as_mut().unwrap().draw_text(
                (point.x as f32, point.y as f32),
                Color::BLACK,
                layout
            );
        }
    }

    // The implementation which will allow this struct to be used to render rbop expressions!
    // There are only three methods which need to be implemented...
    impl<'a> Renderer for Speedy2DRenderer<'a> {
        // The `init` method is called by `draw_all` after computing the layout, but before drawing
        // any glyphs. It can be used to perform any pre-draw setup required; here, that is clearing
        // the screen. 
        fn init(&mut self, _size: rbop::render::Area) {
            self.graphics.as_mut().unwrap().clear_screen(Color::from_rgb(1.0, 1.0, 1.0));
        }

        // The `size` method is used during layout computation. rbop needs to know the size of each
        // glyphs drawn with this renderer, so this method takes a glyph and returns its size.
        //
        // There is no hard requirement that this actually matches the size of the glyphs drawn to
        // the screen; for example, we lie that the cursor has a width of 0, to stop the glyphs
        // around the cursor wobbling when it is moved.
        //
        // The size reduction level starts from 0, and increases by one each time a node is nested
        // inside another in such a way where the resulting glyph should be drawn smaller. For
        // example, when you're writing a power, you tend to write the exponent smaller than the
        // base - so the exponent would have 1 greater size reduction level than the base. You can
        // completely ignore this if you like! (The built-in AsciiRenderer does - ASCII doesn't 
        // give us enough resolution to decrease the size of an exponent.)
        fn size(&mut self, glyph: rbop::render::Glyph, size_reduction_level: u32) -> rbop::render::Area {
            match glyph {
                rbop::render::Glyph::Digit { number } => 
                    self.text_size(&format!("{}", number), size_reduction_level),

                rbop::render::Glyph::Add => self.text_size("+", size_reduction_level),
                rbop::render::Glyph::Subtract => self.text_size("-", size_reduction_level),
                rbop::render::Glyph::Multiply => self.text_size("*", size_reduction_level),
                rbop::render::Glyph::Divide => self.text_size("/", size_reduction_level),

                rbop::render::Glyph::Fraction { inner_width } => rbop::render::Area {
                    width: inner_width,
                    height: 3,
                },

                rbop::render::Glyph::Cursor { height } => rbop::render::Area {
                    // Lie about the cursor width! This means that rbop doesn't make space for the
                    // cursor, so the cursor moving won't cause elements to shift a little bit
                    width: 0,
                    height,
                },
                rbop::render::Glyph::Placeholder => self.text_size("X", size_reduction_level),

                // TODO: not everything's implemented
                rbop::render::Glyph::LeftParenthesis { .. } => todo!(),
                rbop::render::Glyph::RightParenthesis { .. } => todo!(),
                rbop::render::Glyph::Sqrt { .. } => todo!(),
                rbop::render::Glyph::Point => todo!(),
                rbop::render::Glyph::Variable { .. } => todo!(),
                rbop::render::Glyph::FunctionName { .. } => todo!(),
                rbop::render::Glyph::Comma => todo!(),
            }
        }

        // After the layout has been computed, this `draw` method will be called for every glyph.
        // The implementation of this method should draw the passed glyph to the given point.
        fn draw(&mut self, viewport_glyph: rbop::render::ViewportGlyph) {
            // Unpack the given `ViewportGlyph`. These encode quite a bit of information:
            //   - Which glyph it actually is
            //   - The position of the glyph
            //   - How big the glyph is
            //   - How much of the glyph is visible within the viewport, if present
            let ViewportGlyph {
                glyph: SizedGlyph { glyph, size_reduction_level, .. },
                point,
                ..
            } = viewport_glyph;

            // Offset a little bit from the origin
            let point = point.dx(20).dy(20);

            // Match on the glyph to draw  
            match glyph {
                rbop::render::Glyph::Digit { number } =>
                    self.text_draw(&format!("{}", number), point, size_reduction_level),
                rbop::render::Glyph::Add => self.text_draw("+", point, size_reduction_level),
                rbop::render::Glyph::Subtract => self.text_draw("-", point, size_reduction_level),
                rbop::render::Glyph::Multiply => self.text_draw("*", point, size_reduction_level),
                rbop::render::Glyph::Divide => self.text_draw("/", point, size_reduction_level),

                rbop::render::Glyph::Fraction { inner_width } => 
                    self.graphics.as_mut().unwrap().draw_line(
                        (point.x as f32, point.y as f32),
                        (point.x as f32 + inner_width as f32, point.y as f32),
                        3.0,
                        Color::BLACK
                    ),

                rbop::render::Glyph::Cursor { height } =>
                    self.graphics.as_mut().unwrap().draw_line(
                        (point.x as f32, point.y as f32),
                        (point.x as f32, point.y as f32 + height as f32),
                        1.0,
                        Color::BLACK
                    ),
                rbop::render::Glyph::Placeholder => self.text_draw("?", point, size_reduction_level),

                // TODO: not everything's implemented
                rbop::render::Glyph::LeftParenthesis { .. } => todo!(),
                rbop::render::Glyph::RightParenthesis { .. } => todo!(),
                rbop::render::Glyph::Sqrt { .. } => todo!(),    
                rbop::render::Glyph::Point => todo!(),
                rbop::render::Glyph::Variable { .. } => todo!(),
                rbop::render::Glyph::FunctionName { .. } => todo!(),
                rbop::render::Glyph::Comma => todo!(),
            }
        }
    }

    /// The Speedy2D window handler implementation. This also contains the required pieces of the
    /// rbop context, except the renderer - renderer instances are created on-the-fly, trading off
    /// performance for "borrow checker sanity" :P
    struct WindowCalc {
        root: UnstructuredNodeRoot,
        nav_path: rbop::nav::NavPath,
        needs_draw: bool,
    }

    impl WindowCalc {
        /// Create a new window.
        fn new_window() -> Window {
            Window::new_centered("Window Calc", (640, 480))
                .expect("unable to create window")
        }

        #[cfg(target_os = "linux")]
        fn font() -> &'static [u8] {
            include_bytes!("/usr/share/fonts/Arial.ttf")
        }

        #[cfg(target_os = "macos")]
        fn font() -> &'static [u8] {
            include_bytes!("/System/Library/Fonts/Supplemental/Arial.ttf")
        }

        /// Create a new `Speedy2DRenderer` using the given graphics surface.
        fn create_renderer<'a>(&mut self, graphics: Option<&'a mut Graphics2D>) -> Speedy2DRenderer<'a> {
            Speedy2DRenderer {
                graphics,
                font: Font::new(WindowCalc::font()).unwrap(),
            }
        }
    }

    impl WindowHandler for WindowCalc {
        fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
            // Only draw the screen if there was an rbop input since the last draw
            if self.needs_draw {
                graphics.clear_screen(Color::from_rgb(1.0, 1.0, 1.0));

                // A bit funky, but makes the borrow checker happy
                // Means that we drop create_renderer's mutable borrow before using `draw_text` again
                let result_text = {
                    let mut renderer = self.create_renderer(Some(graphics));
                    renderer.draw_all(&self.root, Some(&mut self.nav_path.to_navigator()), None);

                    let result = self.root.upgrade().map(|x| x.evaluate(&EvaluationSettings::default()));
                    renderer.text_layout(&match result {
                        Ok(Ok(number)) => format!("{:?}", number),

                        Ok(Err(error)) => error.to_string(),   
                        Err(error) => error.to_string(),
                    }, 0)
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

        fn on_key_down(&mut self, _helper: &mut WindowHelper<()>, virtual_key_code: Option<VirtualKeyCode>, _scancode: speedy2d::window::KeyScancode) {
            // Create a new renderer - not bound to a particular graphics surface since it's only
            // used for its `size` method
            let mut renderer = self.create_renderer(None);

            // Handle the input key
            let node_to_insert = match virtual_key_code.unwrap() {
                VirtualKeyCode::Key0 => Some(UnstructuredNode::Token(Token::Digit(0))),
                VirtualKeyCode::Key1 => Some(UnstructuredNode::Token(Token::Digit(1))),
                VirtualKeyCode::Key2 => Some(UnstructuredNode::Token(Token::Digit(2))),
                VirtualKeyCode::Key3 => Some(UnstructuredNode::Token(Token::Digit(3))),
                VirtualKeyCode::Key4 => Some(UnstructuredNode::Token(Token::Digit(4))),
                VirtualKeyCode::Key5 => Some(UnstructuredNode::Token(Token::Digit(5))),
                VirtualKeyCode::Key6 => Some(UnstructuredNode::Token(Token::Digit(6))),
                VirtualKeyCode::Key7 => Some(UnstructuredNode::Token(Token::Digit(7))),
                VirtualKeyCode::Key8 => Some(UnstructuredNode::Token(Token::Digit(8))),
                VirtualKeyCode::Key9 => Some(UnstructuredNode::Token(Token::Digit(9))),

                VirtualKeyCode::Plus => Some(UnstructuredNode::Token(Token::Add)),
                VirtualKeyCode::Minus => Some(UnstructuredNode::Token(Token::Subtract)),
                VirtualKeyCode::Asterisk => Some(UnstructuredNode::Token(Token::Multiply)),
                VirtualKeyCode::Slash => Some(UnstructuredNode::Fraction(
                    UnstructuredNodeList::new(),
                    UnstructuredNodeList::new(),
                )),

                VirtualKeyCode::S => Some(UnstructuredNode::Sqrt(
                    UnstructuredNodeList::new(),
                )),
                VirtualKeyCode::P => Some(UnstructuredNode::Power(
                    UnstructuredNodeList::new(),
                )),

                VirtualKeyCode::Left => { self.root.move_left(&mut self.nav_path, &mut renderer, None); None }
                VirtualKeyCode::Right => { self.root.move_right(&mut self.nav_path, &mut renderer, None); None }
                VirtualKeyCode::Down => { self.root.move_down(&mut self.nav_path, &mut renderer, None); None }
                VirtualKeyCode::Up => { self.root.move_up(&mut self.nav_path, &mut renderer, None); None }

                VirtualKeyCode::Backspace => { self.root.delete(&mut self.nav_path, &mut renderer, None); None }

                _ => return,
            };

            if let Some(node) = node_to_insert {
                self.root.insert(
                    &mut self.nav_path,
                    &mut renderer,
                    None,
                    node,
                );
            }

            self.needs_draw = true;
        }
    }

    pub fn main() {
        WindowCalc::new_window().run_loop(WindowCalc {
            root: UnstructuredNodeRoot::new(),
            nav_path: rbop::nav::NavPath::new(vec![0]),
            needs_draw: true,
        })
    }
}

#[cfg(feature = "examples")]
fn main() {
    window_calc::main();
}
