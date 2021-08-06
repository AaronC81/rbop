use crate::node::unstructured::{Navigable, UnstructuredNodeRoot, Upgradable};
use crate::{UnstructuredItem, UnstructuredNodeList};
use crate::nav::NavPath;
use crate::{StructuredNode, UnstructuredNode, Token, render::Renderer};
use crate::renderers::AsciiRenderer;
use alloc::string::{String, ToString};
use alloc::{vec, vec::Vec};
use rust_decimal::Decimal;

macro_rules! uns_list {
    ($($x:expr),* $(,)?) => { UnstructuredNodeList { items: vec![ $($x),* ] } };
}

macro_rules! token {
    (+)             => { UnstructuredNode::Token(Token::Add) };
    (-)             => { UnstructuredNode::Token(Token::Subtract) };
    (*)             => { UnstructuredNode::Token(Token::Multiply) };
    (/)             => { UnstructuredNode::Token(Token::Divide) };
    ($x:literal)    => { UnstructuredNode::Token(Token::Digit($x)) };
}

macro_rules! tokens {
    ($($x:tt) *) => { UnstructuredNodeList { items: vec![ $(token!($x)),* ] } };
}

macro_rules! uns_frac {
    ($t:expr, $b:expr $(,)?) => { UnstructuredNode::Fraction($t, $b) };
}

/// ```text
///       56    
///    34+--
///       78   
/// 12+-----+12
///     90  
/// ```   
fn complex_unstructured_expression() -> UnstructuredNodeRoot {
    UnstructuredNodeRoot { root: uns_list!(
        token!(1),
        token!(2),
        token!(+),
        uns_frac!(
            uns_list!(
                token!(3),
                token!(4),
                token!(+),
                uns_frac!(
                    tokens!(5 6),
                    tokens!(7 8),
                )
            ),
            tokens!(9 0),
        ),
        token!(+),
        token!(1),
        token!(2),
    ) }
}

#[test]
fn test_upgrade() {
    let unstructured = tokens!(1 2 * 3 4 + 5 6 * 7 8);
    
    assert_eq!(
        unstructured.upgrade().unwrap(),
        StructuredNode::Add(
            box StructuredNode::Multiply(
                box StructuredNode::Number(12.into()),
                box StructuredNode::Number(34.into()),
            ),
            box StructuredNode::Multiply(
                box StructuredNode::Number(56.into()),
                box StructuredNode::Number(78.into()),
            ),
        )
    );
}

#[test]
fn test_upgrade_negative_numbers() {
    // Simple case
    assert_eq!(
        tokens!(- 1 2).upgrade().unwrap(),
        StructuredNode::Number((-12).into())
    );

    // Multiple unary minuses
    assert_eq!(
        tokens!(- - - - 1 2).upgrade().unwrap(),
        StructuredNode::Number((12).into())
    );

    // Rendering
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&tokens!(- 1 2).upgrade().unwrap(), None);
    assert_eq!(
        renderer.lines,
        ["-12"]
    );

    // No ambiguity between minus and subtract
    assert_eq!(
        tokens!(1 - - 2).upgrade().unwrap().evaluate().unwrap(),
        Decimal::from(3)
    );
}

#[test]
fn test_disambiguate() {
    let tree = StructuredNode::Multiply(
        box StructuredNode::Number(1.into()),
        box StructuredNode::Multiply(
            box StructuredNode::Number(2.into()),
            box StructuredNode::Number(3.into()),
        ),
    );
    assert_eq!(
        tree.disambiguate().unwrap(),
        StructuredNode::Multiply(
            box StructuredNode::Number(1.into()),
            box StructuredNode::Parentheses(
                box StructuredNode::Multiply(
                    box StructuredNode::Number(2.into()),
                    box StructuredNode::Number(3.into()),
                ),
            ),
        )
    );

    let tree = StructuredNode::Multiply(
        box StructuredNode::Add(
            box StructuredNode::Number(1.into()),
            box StructuredNode::Number(2.into()),
        ),
        box StructuredNode::Add(
            box StructuredNode::Number(3.into()),
            box StructuredNode::Number(4.into()),
        ),
    );
    assert_eq!(
        tree.disambiguate().unwrap(),
        StructuredNode::Multiply(
            box StructuredNode::Parentheses(
                box StructuredNode::Add(
                    box StructuredNode::Number(1.into()),
                    box StructuredNode::Number(2.into()),
                ),
            ),
            box StructuredNode::Parentheses(
                box StructuredNode::Add(
                    box StructuredNode::Number(3.into()),
                    box StructuredNode::Number(4.into()),
                ),
            ),
        )
    );
}

#[test]
fn test_ascii_render() {
    let tree = StructuredNode::Add(
        box StructuredNode::Multiply(
            box StructuredNode::Number(12.into()),
            box StructuredNode::Number(34.into()),
        ),
        box StructuredNode::Multiply(
            box StructuredNode::Number(56.into()),
            box StructuredNode::Number(78.into()),
        ),
    ).disambiguate().unwrap();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&tree, None);
    assert_eq!(
        renderer.lines,
        vec!["12*34+56*78"],
    );

    let tree = StructuredNode::Add(
        box StructuredNode::Add(
            box StructuredNode::Number(12.into()),
            box StructuredNode::Divide(
                box StructuredNode::Add(
                    box StructuredNode::Number(34.into()),
                    box StructuredNode::Divide(
                        box StructuredNode::Number(56.into()),
                        box StructuredNode::Number(78.into()),
                    )
                ),
                box StructuredNode::Number(90.into()),
            ),
        ),
        box StructuredNode::Number(12.into()),
    ).disambiguate().unwrap();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&tree, None);
    assert_eq!(
        renderer.lines,
        vec![
            "      56   ",
            "   34+--   ",
            "      78   ",
            "12+-----+12",
            "    90     "
        ],
    );

    let tree = complex_unstructured_expression();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&tree, None);
    assert_eq!(
        renderer.lines,
        vec![
            "      56   ",
            "   34+--   ",
            "      78   ",
            "12+-----+12",
            "    90     "
        ],
    );

    // Basic cursor
    let tree = complex_unstructured_expression();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&tree, Some(&mut NavPath::new(vec![3, 0, 3, 1, 1]).to_navigator()));
    assert_eq!(
        renderer.lines,
        vec![
            "      56    ",
            "   34+---   ",
            "      7|8   ",
            "12+------+12",
            "     90     "
        ],
    );

    // Cursor matches adjacent size
    let tree = complex_unstructured_expression();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&tree, Some(&mut NavPath::new(vec![3, 0, 3]).to_navigator()));
    assert_eq!(
        renderer.lines,
        vec![
            "      |56   ",
            "   34+|--   ",
            "      |78   ",
            "12+------+12",
            "     90     "
        ],
    );
}

#[test]
fn test_navigation() {
    let mut unstructured = uns_list!(
        token!(1),
        token!(2),
        token!(*),
        token!(3),
        token!(4),
        token!(+),
        uns_frac!(
            tokens!(5 6),
            tokens!(7 8),
        )
    );

    // Path 1: beginning
    let mut path = NavPath::new(vec![0]);
    let result = {
        let (node, i) = unstructured.navigate(&mut path.to_navigator());
        let node_ptr: *mut UnstructuredNodeList = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (&mut unstructured as *mut UnstructuredNodeList, 0)
    );

    // Path 2: middle
    let mut path = NavPath::new(vec![3]);
    let result = {
        let (node, i) = unstructured.navigate(&mut path.to_navigator());
        let node_ptr: *mut UnstructuredNodeList = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (&mut unstructured as *mut UnstructuredNodeList, 3)
    );

    // Path 3: nested
    let mut path = NavPath::new(vec![6, 1, 1]);
    let result = {
        let (node, i) = unstructured.navigate(&mut path.to_navigator());
        (node.clone(), i)
    };
    assert_eq!(
        result,
        (tokens!(7 8), 1)
    );
}

#[test]
fn test_movement() {
    let mut node = complex_unstructured_expression();
    let mut nav_path = NavPath::new(vec![0]);
    let mut renderer = AsciiRenderer::default();

    // Go all the way to the right
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![1]));

    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3]));

    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));

    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));

    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 2]));

    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 4]));

    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![4]));

    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![5]));

    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![7]));

    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![7]));

    // Now go back to the left
    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![6]));

    node.move_left(&mut nav_path);
    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![4]));

    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 4]));

    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 2]));

    node.move_left(&mut nav_path);
    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));

    node.move_left(&mut nav_path);
    node.move_left(&mut nav_path);
    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));

    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3]));

    node.move_left(&mut nav_path);
    node.move_left(&mut nav_path);
    node.move_left(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![0]));

    // Move into a fraction to test vertical movement
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));
    
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));
    
    // Now in a fraction, test vertical movement
    node.move_up(&mut nav_path, &mut renderer);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_down(&mut nav_path, &mut renderer);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 1, 0]));

    node.move_up(&mut nav_path, &mut renderer);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_down(&mut nav_path, &mut renderer);
    node.move_down(&mut nav_path, &mut renderer);
    assert_eq!(nav_path, NavPath::new(vec![3, 1, 2]));

    node.move_up(&mut nav_path, &mut renderer);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));
}

#[test]
fn test_modification() {
    let mut node = complex_unstructured_expression();
    let mut nav_path = NavPath::new(vec![0]);

    node.insert(&mut nav_path, UnstructuredNode::Token(Token::Digit(1)));
    assert_eq!(nav_path, NavPath::new(vec![1]));

    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 0]));

    node.move_right(&mut nav_path);
    node.move_right(&mut nav_path);
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 2]));

    node.insert(&mut nav_path, UnstructuredNode::Token(Token::Add));
    node.insert(&mut nav_path, UnstructuredNode::Fraction(
        UnstructuredNodeList { items: vec![] },
        UnstructuredNodeList { items: vec![] },
    ));
    node.insert(&mut nav_path, UnstructuredNode::Token(Token::Digit(9)));
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 3, 0, 1]));

    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(&node, Some(&mut nav_path.to_navigator()));
    assert_eq!(
        renderer.lines,
        vec![
            "       9| 56   ",
            "    34+--+--   ",
            "          78   ",
            "112+--------+12",
            "       90      "
        ],
    );

    // Now try deleting some bits
    node.delete(&mut nav_path);
    node.delete(&mut nav_path);
    node.delete(&mut nav_path);

    renderer.draw_all(&node, Some(&mut nav_path.to_navigator()));
    assert_eq!(
        renderer.lines,
        vec![
            "        56   ",
            "    34|+--   ",
            "        78   ",
            "112+------+12",
            "      90     "
        ],
    );
}
