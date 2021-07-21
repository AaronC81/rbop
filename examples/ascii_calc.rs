#![feature(box_syntax)]

use rbop::Token;
use rbop::{renderers::AsciiRenderer, Node, nav::NavPath, render::Renderer};
use core::time;
use std::error::Error;
use std::io::{Write, stdin, stdout};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn main() -> Result<(), Box<dyn Error>> {
    std::panic::set_hook(box |info| {
        println!("Panic!");
        println!("{:?}", info.payload().downcast_ref::<&str>());
        std::thread::sleep(time::Duration::from_secs(2));
    });
        
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode()?;

    let mut node = Node::Unstructured(vec![]);
    let mut renderer = AsciiRenderer::default();
    let mut nav_path = NavPath::new(vec![0]);

    let keys = stdin.keys();

    for k in keys {
        match k? {
            Key::Char('q') => break,
            Key::Char(d) if d.is_digit(10) =>
                node.insert(&mut nav_path, Node::Token(Token::Digit(d.to_digit(10).unwrap() as u8))),

            Key::Char('+') => node.insert(&mut nav_path, Node::Token(Token::Add)),
            Key::Char('-') => node.insert(&mut nav_path, Node::Token(Token::Subtract)),
            Key::Char('x') => node.insert(&mut nav_path, Node::Token(Token::Multiply)),
            Key::Char('/') => node.insert(&mut nav_path, Node::Divide(
                box Node::Unstructured(vec![]),
                box Node::Unstructured(vec![]),
            )),

            Key::Char('s') => node.insert(&mut nav_path, Node::Sqrt(
                box Node::Unstructured(vec![]),
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

        renderer.draw_all(node.clone(), Some(&mut nav_path.to_navigator()));

        for line in renderer.lines.iter() {
            write!(stdout, "{}\r\n", line)?;
        }

        stdout.flush()?;
    };

    Ok(())
}
