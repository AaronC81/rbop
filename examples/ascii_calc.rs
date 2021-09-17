// This example shows how to maintain an rbop state, feed it input, and evaluate the current node
// tree to produce a result.
//
// This example doesn't implement a renderer - we're using the `AsciiRenderer` built into rbop.
// For a more complete example which does implement its own renderer, refer to `window_calc`.

#![feature(box_syntax)]
#![feature(backtrace)]
#![feature(const_panic)]

// Enforce `examples` feature is passed ------------------------------------------------------------
#[cfg(not(feature = "examples"))]
mod ascii_calc {
    pub const CHECK: usize = panic!("you must enable the `examples` feature to compile examples.");
}

#[cfg(not(feature = "examples"))]
fn main() {
    println!("{}", ascii_calc::CHECK);
}
// -------------------------------------------------------------------------------------------------

use std::error::Error;

#[cfg(feature = "examples")]
mod ascii_calc {
    use core::time;
    use std::backtrace::Backtrace;
    use std::io::{Write, stdin, stdout};
    use std::error::Error;

    use termion::event::Key;
    use termion::input::TermRead;
    use termion::raw::IntoRawMode;

    use rbop::{Token, UnstructuredNode, UnstructuredNodeList};
    use rbop::node::unstructured::{UnstructuredNodeRoot, Upgradable};
    use rbop::{renderers::AsciiRenderer, nav::NavPath, render::Renderer};

    pub fn main() -> Result<(), Box<dyn Error>> {
        // Add a nice panic handler - this example is great for testing new rbop functionality, so
        // useful panic output is valuable
        std::panic::set_hook(box |info| {
            println!("Panic!");
            println!("{:?}", info.payload().downcast_ref::<&str>());
    
            println!("{}", Backtrace::force_capture());
    
            std::thread::sleep(time::Duration::from_secs(2));
        });
            
        // Terminal setup using termion
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode()?;
    
        // Now set up all the pieces of an rbop state!
        //
        // There are two things which you will always need to use rbop:
        //   - An instance of `UnstructuredNodeRoot`. This is a representation of the user's input,
        //     which builds up a calculation to display and evaluate.
        //   - An instance of something implementing the `Renderer` trait. A renderer implementation
        //     can provide the information necessary for rbop to convert a tree of nodes into a list
        //     of simple glyphs, and then draw these glyphs to some kind of graphics surface.
        //
        // If your node tree needs to be editable (i.e. you're not just using rbop to render
        // pre-existing expressions), you will also need an instance of `NavPath`. This describes
        // where the cursor is in the associated unstructured node tree.
        //
        // Some use-cases may also need a `Viewport`, which describes the bounds of the graphics
        // surface to allow rbop to implement scrolling to keep the cursor on screen, and clipping
        // glyphs which are not visible in the viewport. For this example, we are not using a
        // viewport, so rbop will assume an infinitely-sized area. (We could pass a viewport based
        // on the size of the terminal, but this becomes a little tricky since the terminal could be
        // resized on-the-fly. Viewports are more aimed at embedded use-cases with small, fixed-size
        // displays.)
        let mut root = UnstructuredNodeRoot { root: UnstructuredNodeList { items: vec![] } };
        let mut renderer = AsciiRenderer::default();
        let mut nav_path = NavPath::new(vec![0]);
    
        // This is an infinite loop which iterates when a key is pressed
        for k in stdin.keys() {
            // Match the pressed key. Most of the these keys, but not all, will insert a new node,
            // so to avoid duplicating a `root.insert` call, we return Option<UnstructuredNode>. If
            // `Some(node)`, we insert `node` afterwards; if `None`, that key doesn't need to insert
            // anything.
            let node_to_insert = match k? {
                Key::Char('q') => break,
                Key::Char(d) if d.is_digit(10) =>
                    Some(UnstructuredNode::Token(Token::Digit(d.to_digit(10).unwrap() as u8))),
    
                Key::Char('+') => Some(UnstructuredNode::Token(Token::Add)),
                Key::Char('-') => Some(UnstructuredNode::Token(Token::Subtract)),
                Key::Char('x') => Some(UnstructuredNode::Token(Token::Multiply)),
                Key::Char('/') => Some(UnstructuredNode::Fraction(
                    UnstructuredNodeList { items: vec![] },
                    UnstructuredNodeList { items: vec![] },
                )),
    
                Key::Char('s') => Some(UnstructuredNode::Sqrt(
                    UnstructuredNodeList { items: vec![] }
                )),
                Key::Char('^') => Some(UnstructuredNode::Power(
                    UnstructuredNodeList { items: vec![] }
                )),
    
                Key::Left => { root.move_left(&mut nav_path, &mut renderer, None); None }
                Key::Right => { root.move_right(&mut nav_path, &mut renderer, None); None }
                Key::Down => { root.move_down(&mut nav_path, &mut renderer, None); None },
                Key::Up => { root.move_up(&mut nav_path, &mut renderer, None); None },
    
                Key::Backspace => { root.delete(&mut nav_path, &mut renderer, None); None },
                _ => None,
            };

            // If the pressed key needs to insert a node, insert it
            if let Some(new_node) = node_to_insert {
                root.insert(&mut nav_path, &mut renderer, None, new_node);
            }
    
            // Move the cursor back up to the top right
            write!(stdout,
                "{}{}",
                termion::cursor::Goto(1, 1),
                termion::clear::All)
                 .unwrap();
    
            // Ask the renderer to draw the current node tree!
            //
            // The `draw_all` method is essentially a "do-it-all-in-one" method call, which wraps
            // up rbop's important tasks of:
            //   - Computing a layout of glyphs from the nodes
            //   - Initialising the graphics surface
            //   - Drawing glyphs to the graphics surface
            renderer.draw_all(&root, Some(&mut nav_path.to_navigator()), None);
    
            // `AsciiRenderer` does not draw straight to the screen, it draws to a buffer of lines
            // of text - so print these to the console
            for line in renderer.lines.iter() {
                write!(stdout, "{}\r\n", line)?;
            }
    
            write!(stdout, "\r\n===================================\r\n")?;
    
            // Next we'd like to evaluate this expression and print a result. Unstructured nodes do
            // not express any precedence information, so first we need to *upgrade* it to a
            // structured node tree. If the input tree contains parse errors, this will return None.
            match root.upgrade() {
                Ok(upgraded) => {
                    // If the upgrade succeeded, we now have a valid structured node tree. We can
                    // try to evaluate this now, which might fail if there are maths errors or
                    // similar.
                    match upgraded.evaluate() {
                        Ok(result) => write!(stdout, "{}", result)?,
                        Err(err) => write!(stdout, "Evaluation error: {}", err)?,
                    }
                    
                    // Also print the node tree
                    write!(stdout, "\r\n\r\n{:?}", upgraded)?;
                },
                Err(err) => write!(stdout, "Parse error: {}", err)?,
            };
    
            // Ensure everything is printed
            stdout.flush()?;
        };
    
        Ok(())    
    }
}

#[cfg(feature = "examples")]
fn main() -> Result<(), Box<dyn Error>> {
    ascii_calc::main()
}
