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

#[cfg(feature = "examples")]
mod ascii_calc {
    use termion::event::Key;
    use termion::input::TermRead;
    use termion::raw::IntoRawMode;
    use rbop::{Token, UnstructuredNode, UnstructuredNodeList};
    use rbop::node::unstructured::{UnstructuredNodeRoot, Upgradable};
    use rbop::{renderers::AsciiRenderer, nav::NavPath, render::Renderer};
    use core::time;
    use std::backtrace::Backtrace;
    use std::error::Error;
    use std::io::{Write, stdin, stdout};

    fn main() -> Result<(), Box<dyn Error>> {
        std::panic::set_hook(box |info| {
            println!("Panic!");
            println!("{:?}", info.payload().downcast_ref::<&str>());
    
            println!("{}", Backtrace::force_capture());
    
            std::thread::sleep(time::Duration::from_secs(2));
        });
            
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode()?;
    
        let mut node = UnstructuredNodeRoot { root: UnstructuredNodeList { items: vec![] } };
        let mut renderer = AsciiRenderer::default();
        let mut nav_path = NavPath::new(vec![0]);
    
        let keys = stdin.keys();
    
        for k in keys {
            match k? {
                Key::Char('q') => break,
                Key::Char(d) if d.is_digit(10) =>
                    node.insert(&mut nav_path, UnstructuredNode::Token(Token::Digit(d.to_digit(10).unwrap() as u8))),
    
                Key::Char('+') => node.insert(&mut nav_path, UnstructuredNode::Token(Token::Add)),
                Key::Char('-') => node.insert(&mut nav_path, UnstructuredNode::Token(Token::Subtract)),
                Key::Char('x') => node.insert(&mut nav_path, UnstructuredNode::Token(Token::Multiply)),
                Key::Char('/') => node.insert(&mut nav_path, UnstructuredNode::Fraction(
                    UnstructuredNodeList { items: vec![] },
                    UnstructuredNodeList { items: vec![] },
                )),
    
                Key::Char('s') => node.insert(&mut nav_path, UnstructuredNode::Sqrt(
                    UnstructuredNodeList { items: vec![] }
                )),
    
                Key::Left => node.move_left(&mut nav_path),
                Key::Right => node.move_right(&mut nav_path),
                Key::Down => node.move_down(&mut nav_path, &mut renderer),
                Key::Up => node.move_up(&mut nav_path, &mut renderer),
    
                Key::Backspace => node.delete(&mut nav_path),
                _ => (),
            }
    
            write!(stdout,
                "{}{}",
                termion::cursor::Goto(1, 1),
                termion::clear::All)
                 .unwrap();
    
            renderer.draw_all(&node, Some(&mut nav_path.to_navigator()));
    
            for line in renderer.lines.iter() {
                write!(stdout, "{}\r\n", line)?;
            }
    
            write!(stdout, "\r\n===================================\r\n")?;
    
            match node.upgrade() {
                Ok(upgraded) => match upgraded.evaluate() {
                    Ok(result) => write!(stdout, "{}", result)?,
                    Err(err) => write!(stdout, "Evaluation error: {}", err)?,
                },
                Err(err) => write!(stdout, "Parse error: {}", err)?,
            };
    
            stdout.flush()?;
        };
    
        Ok(())    
    }
}

#[cfg(feature = "examples")]
fn main() -> Result<(), Box<dyn Error>> {
    ascii_calc::main()
}
