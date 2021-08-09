use core::str::FromStr;

use crate::node::unstructured::{Navigable, UnstructuredNodeRoot, Upgradable};
use crate::render::{Area, CalculatedPoint, Viewport};
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
    (.)             => { UnstructuredNode::Token(Token::Point) };
    ($x:literal)    => { UnstructuredNode::Token(Token::Digit($x)) };
}

macro_rules! tokens {
    ($($x:tt) *) => { UnstructuredNodeList { items: vec![ $(token!($x)),* ] } };
}

macro_rules! uns_frac {
    ($t:expr, $b:expr $(,)?) => { UnstructuredNode::Fraction($t, $b) };
}

macro_rules! render {
    ($n:expr, $p:expr, $v:expr $(,)?) => { {
        let mut renderer = AsciiRenderer::default();
        renderer.draw_all(&$n, $p, $v);
        renderer.lines
    } };

    ($n:expr, $p:expr $(,)?) => { render!($n, $p, None) };

    ($n:expr $(,)?) => { render!($n, None, None) };
}

macro_rules! dec {
    ($l:literal) => { Decimal::from_str(stringify!($l)).unwrap() };
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
    renderer.draw_all(&tokens!(- 1 2).upgrade().unwrap(), None, None);
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
fn test_decimals() {
    // Upgrading
    assert_eq!(
        tokens!(1 . 2).upgrade().unwrap(),
        StructuredNode::Number(dec!(1.2))
    );
    assert_eq!(
        tokens!(1 2 3 . 4 5).upgrade().unwrap(),
        StructuredNode::Number(dec!(123.45))
    );

    // Rendering as unstructured
    assert_eq!(
        render!(tokens!(1 . 2)),
        vec!["1.2"],
    );

    // Rendering as structured
    assert_eq!(
        render!(tokens!(1 . 2).upgrade().unwrap()),
        vec!["1.2"],
    );

    // Evaluation
    assert_eq!(
        tokens!(0 . 3 - 0 . 1).upgrade().unwrap().evaluate().unwrap(),
        dec!(0.2)
    );

    // Check we accept the "3." form 
    assert_eq!(
        tokens!(3 . + 2 .).upgrade().unwrap().evaluate().unwrap(),
        dec!(5)
    );

    // Check we error on multiple decimal points
    assert!(matches!(
        tokens!(1 2 . 3 4 . 5 6).upgrade(),
        Err(_)
    ))
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
    assert_eq!(
        render!(tree),
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
    assert_eq!(
        render!(tree),
        vec![
            "      56   ",
            "   34+--   ",
            "      78   ",
            "12+-----+12",
            "    90     "
        ],
    );

    let tree = complex_unstructured_expression();
    assert_eq!(
        render!(tree),
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
    assert_eq!(
        render!(tree, Some(&mut NavPath::new(vec![3, 0, 3, 1, 1]).to_navigator())),
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
    assert_eq!(
        render!(tree, Some(&mut NavPath::new(vec![3, 0, 3]).to_navigator())),
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
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![1]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 2]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 4]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![5]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![7]));

    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![7]));

    // Now go back to the left
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![6]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 4]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 2]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));

    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3]));

    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    node.move_left(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![0]));

    // Move into a fraction to test vertical movement
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 0]));
    
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));
    
    // Now in a fraction, test vertical movement
    node.move_up(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_down(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 1, 0]));

    node.move_up(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3, 0, 0]));

    node.move_down(&mut nav_path, &mut renderer, None);
    node.move_down(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 1, 2]));

    node.move_up(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![3, 0, 3]));
}

#[test]
fn test_modification() {
    let mut node = complex_unstructured_expression();
    let mut nav_path = NavPath::new(vec![0]);
    let mut renderer = AsciiRenderer::default();

    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Digit(1)));
    assert_eq!(nav_path, NavPath::new(vec![1]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 0]));

    node.move_right(&mut nav_path, &mut renderer, None);
    node.move_right(&mut nav_path, &mut renderer, None);
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 2]));

    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Add));
    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Fraction(
        UnstructuredNodeList { items: vec![] },
        UnstructuredNodeList { items: vec![] },
    ));
    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Digit(9)));
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 3, 0, 1]));

    assert_eq!(
        render!(node, Some(&mut nav_path.to_navigator())),
        vec![
            "       9| 56   ",
            "    34+--+--   ",
            "          78   ",
            "112+--------+12",
            "       90      "
        ],
    );

    // Now try deleting some bits
    node.delete(&mut nav_path, &mut renderer, None);
    node.delete(&mut nav_path, &mut renderer, None);
    node.delete(&mut nav_path, &mut renderer, None);

    assert_eq!(
        render!(node, Some(&mut nav_path.to_navigator())),
        vec![
            "        56   ",
            "    34|+--   ",
            "        78   ",
            "112+------+12",
            "      90     "
        ],
    );
}

#[test]
fn test_viewport() {
    // No viewport should show everything
    assert_eq!(
        render!(tokens!(1 2 3 4 5)),
        vec!["12345"],
    );

    // Very large viewport should show everything
    assert_eq!(
        render!(tokens!(1 2 3 4 5), None, Some(&Viewport::new(Area::new(10, 3)))),
        vec![
            "12345     ",
            "          ",
            "          ",
        ],
    );

    // Perfectly sized viewport should show everything
    assert_eq!(
        render!(tokens!(1 2 3 4 5), None, Some(&Viewport::new(Area::new(5, 1)))),
        vec!["12345"],
    );

    // Viewport at origin should prune what's to the right
    assert_eq!(
        render!(tokens!(1 2 3 4 5), None, Some(&Viewport::new(Area::new(2, 1)))),
        vec!["12"],
    );

    // Viewport in middle should prune what's to the left and right
    assert_eq!(
        render!(tokens!(1 2 3 4 5), None, Some(&Viewport {
            size: Area::new(2, 1),
            offset: CalculatedPoint { x: 1, y: 0 },
        })),
        vec!["23"],
    );

    // Viewport should clip glyphs which don't completely fit
    assert_eq!(
        render!(
            uns_list!(
                UnstructuredNode::Fraction(
                    tokens!(1 + 2 + 3 + 4),
                    tokens!(5 + 6 + 7 + 8),
                )
            ),
            None,
            Some(&Viewport {
                size: Area::new(3, 3),
                offset: CalculatedPoint { x: 2, y: 0 }
            })
        ),
        vec![
            "2+3",
            "---",
            "6+7",
        ]
    );
}
