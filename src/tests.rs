use core::assert_matches::assert_matches;
use core::str::FromStr;

use crate::error::NodeError;
use crate::node::function::Function;
use crate::node::simplified::{Simplifiable, SimplifiedNode};
use crate::node::structured::{EvaluationSettings, AngleUnit};
use crate::node::unstructured::{Navigable, Serializable, UnstructuredNodeRoot, Upgradable};
use crate::render::{Area, CalculatedPoint, Layoutable, Viewport, LayoutComputationProperties, Glyph};
use crate::{Number, UnstructuredNodeList};
use crate::number::DecimalAccuracy;
use crate::nav::NavPath;
use crate::{StructuredNode, UnstructuredNode, Token, render::Renderer};
use crate::renderers::AsciiRenderer;
use alloc::vec;
use rust_decimal::Decimal;
use test::{Bencher, black_box};

macro_rules! uns_list {
    ($($x:expr),* $(,)?) => { UnstructuredNodeList { items: vec![ $($x),* ] } };
}

macro_rules! token {
    (+)             => { UnstructuredNode::Token(Token::Add) };
    (-)             => { UnstructuredNode::Token(Token::Subtract) };
    (*)             => { UnstructuredNode::Token(Token::Multiply) };
    (/)             => { UnstructuredNode::Token(Token::Divide) };
    (.)             => { UnstructuredNode::Token(Token::Point) };
    (var $v:ident)  => { UnstructuredNode::Token(Token::Variable(stringify!($v).chars().nth(0).unwrap())) };
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

macro_rules! rat {
    ($n:literal)             => { Number::Rational($n, 1)  };
    ($n:literal, $d:literal) => { Number::Rational($n, $d) };
}

macro_rules! dec {
    ($l:literal) => { Number::Decimal(Decimal::from_str(stringify!($l)).unwrap(), DecimalAccuracy::Exact) };
}

macro_rules! dec_approx {
    ($l:literal) => { Number::Decimal(Decimal::from_str(stringify!($l)).unwrap(), DecimalAccuracy::Approximation) };
}

macro_rules! reserialize {
    ($e:expr) => {
        UnstructuredNodeRoot::deserialize(
            &mut $e.serialize().into_iter()
        ).unwrap()
    };
}

macro_rules! reduce {
    ($n:expr) => {
        {
            let mut nodes = $n;
            assert!(matches!(nodes.reduce(), Ok(_)));
            nodes
        }
    };
}

macro_rules! simplify {
    ($t:expr) => {
        {
            let mut n = $t.upgrade().unwrap().simplify().flatten();
            n.sort();
            n
        }
    };
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
        tokens!(1 - - 2).upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        rat!(3)
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
    assert_eq!(
        tokens!(1 2 . 0 0 0 0 1 3).upgrade().unwrap(),
        StructuredNode::Number(dec!(12.000013))
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
        tokens!(0 . 3 - 0 . 1).upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        dec!(0.2)
    );

    // Very large shouldn't panic
    assert!(matches!(
        tokens!(1 2 3 4 5 1 2 3 4 5 1 2 3 4 5 1 2 3 4 5 1 2 3 4 5 1 2 3 4 5 1 2 3 4 5 1 2 3 4 5).upgrade(),
        Err(NodeError::Overflow),
    ));

    // Check we accept the "3." form, and that it becomes a decimal rather than a rational
    assert_eq!(
        tokens!(3 . + 2 .).upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
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

    // Rationals which aren't whole should be rendered as fractions
    let tree = StructuredNode::Add(
        box StructuredNode::Number(rat!(2, 3)),
        box StructuredNode::Number(rat!(1)),
    );
    assert_eq!(
        render!(tree),
        vec![
            "2  ",
            "-+1",
            "3  ",
        ]
    )
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
        UnstructuredNodeList::new(),
        UnstructuredNodeList::new(),
    ));
    node.insert(&mut nav_path, &mut renderer, None, UnstructuredNode::Token(Token::Digit(9)));
    assert_eq!(nav_path, NavPath::new(vec![4, 0, 3, 0, 1]));

    assert_eq!(
        render!(node, Some(&mut nav_path.to_navigator())),
        vec![
            "       9| 56   ",
            "    34+--+--   ",
            "        X 78   ",
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

#[test]
fn test_parentheses() {
    let nodes = uns_list!(
        token!(1),
        token!(+),
        UnstructuredNode::Parentheses(uns_list!(
            UnstructuredNode::Fraction(
                uns_list!(UnstructuredNode::Fraction(
                    uns_list!(token!(2)), uns_list!(token!(3)))
                ),
                uns_list!(UnstructuredNode::Parentheses(uns_list!(token!(4)))),
            )
        ))
    );

    assert_eq!(
        render!(nodes, None, None),
        vec![
            "  / 2 \\",
            "  | - |",
            "  | 3 |",
            "1+|---|",
            "  \\(4)/"
        ]
    )
}

#[test]
fn test_variables() {
    let nodes = uns_list!(
        token!(var x),
        token!(+),
        UnstructuredNode::Fraction(
            uns_list!(
                token!(1),
                token!(+),
                token!(var x),
            ),
            uns_list!(
                token!(var y),
            )
        )
    );

    assert_eq!(
        nodes.upgrade()
            .unwrap()
            .substitute_variable('x', &StructuredNode::Number(dec!(9)))
            .substitute_variable('y', &StructuredNode::Number(dec!(2)))
            .evaluate(&EvaluationSettings::default())
            .unwrap(),
        dec!(14)
    )
}

#[test]
fn test_implicit_multiply() {
    // 2x = 2 * x
    assert_eq!(
        uns_list!(
            token!(2),
            token!(var x),
        ).upgrade().unwrap(),
        StructuredNode::Multiply(
            box StructuredNode::Number(rat!(2)),
            box StructuredNode::Variable('x'),
        )
    );

    // 0.5x = 0.5 * x
    assert_eq!(
        uns_list!(
            token!(0),
            token!(.),
            token!(5),
            token!(var x),
        ).upgrade().unwrap(),
        StructuredNode::Multiply(
            box StructuredNode::Number(dec!(0.5)),
            box StructuredNode::Variable('x'),
        )
    );

    // 2(1+x) = 2 * (1 + x)
    assert_eq!(
        uns_list!(
            token!(2),
            UnstructuredNode::Parentheses(uns_list!(
                token!(1),
                token!(+),
                token!(var x),
            )),
        ).upgrade().unwrap(),
        StructuredNode::Multiply(
            box StructuredNode::Number(rat!(2)),
            box StructuredNode::Parentheses(
                box StructuredNode::Add(
                    box StructuredNode::Number(rat!(1)),
                    box StructuredNode::Variable('x'),
                )
            )
        )
    );

    // xyz + 2 = ((x * y) * z) + 2
    assert_eq!(
        uns_list!(
            token!(var x),
            token!(var y),
            token!(var z),
            token!(+),
            token!(2),
        ).upgrade().unwrap(),
        StructuredNode::Add(
            box StructuredNode::Multiply(
                box StructuredNode::Variable('x'),
                box StructuredNode::Multiply(
                    box StructuredNode::Variable('y'),
                    box StructuredNode::Variable('z'),
                ),
            ),
            box StructuredNode::Number(rat!(2)),
        )
    );
}

#[test]
fn test_power() {
    let tree = StructuredNode::Power(
        box StructuredNode::Number(dec!(12)),
        box StructuredNode::Number(dec!(3)),
    ).disambiguate().unwrap();
    assert_eq!(
        render!(tree),
        vec![
            "  3",
            "12 ",
        ],
    );

    let tree = StructuredNode::Add(
        box StructuredNode::Add(
            box StructuredNode::Add(
                box StructuredNode::Power(
                    box StructuredNode::Number(dec!(12)),
                    box StructuredNode::Number(dec!(3)),
                ),
                box StructuredNode::Power(
                    box StructuredNode::Number(dec!(45)),
                    box StructuredNode::Divide(
                        box StructuredNode::Number(dec!(67)),
                        box StructuredNode::Number(dec!(8)),
                    ),
                ),
            ),
            box StructuredNode::Number(dec!(9)),
        ),
        box StructuredNode::Number(1.into()),
    ).disambiguate().unwrap();
    assert_eq!(
        render!(tree),
        vec![
            "      67    ",
            "      --    ",
            "  3    8    ",
            "12 +45  +9+1",
        ],
    );

    assert_eq!(
        uns_list!(
            token!(2),
            UnstructuredNode::Power(tokens!(3))
        ).upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        rat!(8),
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        token!(1),
        token!(2),
        UnstructuredNode::Power(tokens!(3 4))
    ) };
    assert_eq!(
        render!(tree),
        vec![
            "  34",
            "12  ",
        ],
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        UnstructuredNode::Fraction(tokens!(1 2), tokens!(3 4)),
        token!(+),
        token!(5),
        UnstructuredNode::Power(tokens!(6 7))
    ) };
    assert_eq!(
        render!(tree),
        vec![
            "12  67",
            "--+5  ",
            "34    ",
        ],
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        UnstructuredNode::Parentheses(uns_list!(
            UnstructuredNode::Fraction(tokens!(1 2), tokens!(3 4)),
            token!(+),
            token!(5),
        )),
        UnstructuredNode::Power(tokens!(6 7))
    ) };
    assert_eq!(
        render!(tree),
        vec![
            "      67",
            "/12  \\  ",
            "|--+5|  ",
            "\\34  /  ",
        ],
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        token!(1),
        token!(+),
        token!(2),
        UnstructuredNode::Power(tokens!(2)),
        UnstructuredNode::Power(tokens!(3)),
        UnstructuredNode::Power(tokens!(4)),
        token!(+),
        token!(1),
    ) };
    assert_eq!(
        render!(tree),
        vec![
            "     4  ",
            "    3   ",
            "   2    ",
            "1+2   +1",
        ],
    );
    assert_eq!(
        tree.upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        rat!(16777218)
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        token!(4),
        UnstructuredNode::Power(uns_list!(UnstructuredNode::Fraction(tokens!(1), tokens!(2)))),
    ) };
    assert_eq!(
        render!(tree),
        vec![
            " 1",
            " -",
            " 2",
            "4 ",
        ],
    );
    assert_eq!(
        tree.upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        rat!(2)
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        token!(8),
        UnstructuredNode::Power(uns_list!(
            token!(-),
            UnstructuredNode::Fraction(tokens!(2), tokens!(3))
        )),
    ) };
    assert_eq!(
        render!(tree),
        vec![
            "  2",
            " --",
            "  3",
            "8  ",
        ],
    );
    assert_eq!(
        tree.upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        rat!(1, 4)
    );

    let tree = UnstructuredNodeRoot { root: uns_list!(
        token!(7),
        UnstructuredNode::Power(uns_list!(
            UnstructuredNode::Fraction(tokens!(1), tokens!(2))
        )),
    ) };
    assert_eq!(
        render!(tree),
        vec![
            " 1",
            " -",
            " 2",
            "7 ",
        ],
    );
    assert!(matches!(
        tree.upgrade().unwrap().evaluate(&EvaluationSettings::default()).unwrap(),
        Number::Decimal(_, _),
    ));
}

#[test]
fn test_serialize() {
    // Core stuff
    assert_eq!(
        reserialize!(complex_unstructured_expression()),
        complex_unstructured_expression(),
    );

    // Variables
    let e = UnstructuredNodeRoot { root: uns_list!(
        token!(var x),
        token!(+),
        token!(2),
        token!(+),
        token!(var y),
    ) };
    assert_eq!(
        reserialize!(e),
        e
    );
}

#[test]
fn test_simplify_structured() {
    assert_eq!(
        simplify!(tokens!(1 2 + 5 6 + 3 4)),
        SimplifiedNode::Add(vec![
            SimplifiedNode::Number(rat!(12)),
            SimplifiedNode::Number(rat!(34)),
            SimplifiedNode::Number(rat!(56)),
        ])
    );

    assert_eq!(
        simplify!(tokens!(1 2 + 5 6 * 2 + 3 4)),
        SimplifiedNode::Add(vec![
            SimplifiedNode::Number(rat!(12)),
            SimplifiedNode::Number(rat!(34)),
            SimplifiedNode::Multiply(vec![
                SimplifiedNode::Number(rat!(2)),
                SimplifiedNode::Number(rat!(56)),
            ]),
        ])
    );

    // 1 + ( 2 + ( 3 * ( 4 * 5 ) * 6 ) + ( 7 + 8 ) ) * 9
    // simplifies, flattens and sorts to 1 + (2 + 7 + 8 + (3 * 4 * 5 * 6)) * 9
    assert_eq!(
        simplify!(uns_list!(
            token!(1),
            token!(+),
            UnstructuredNode::Parentheses(uns_list!(
                token!(2),
                token!(+),
                UnstructuredNode::Parentheses(uns_list!(
                    token!(3),
                    token!(*),
                    UnstructuredNode::Parentheses(uns_list!(
                        token!(4),
                        token!(*),
                        token!(5),
                    )),
                    token!(*),
                    token!(6),
                )),
                token!(+),
                UnstructuredNode::Parentheses(uns_list!(
                    token!(7),
                    token!(+),
                    token!(8),
                )),
            )),
            token!(*),
            token!(9),
        )),
        SimplifiedNode::Add(vec![
            SimplifiedNode::Number(rat!(1)),
            SimplifiedNode::Multiply(vec![
                SimplifiedNode::Number(rat!(9)),
                SimplifiedNode::Add(vec![
                    SimplifiedNode::Number(rat!(2)),
                    SimplifiedNode::Number(rat!(7)),
                    SimplifiedNode::Number(rat!(8)),
                    SimplifiedNode::Multiply(vec![
                        SimplifiedNode::Number(rat!(3)),
                        SimplifiedNode::Number(rat!(4)),
                        SimplifiedNode::Number(rat!(5)),
                        SimplifiedNode::Number(rat!(6)),
                    ]),
                ]),
            ]),
        ])
    );

    assert_eq!(
        simplify!(tokens!(1 - 5)),
        SimplifiedNode::Add(vec![
            SimplifiedNode::Number(rat!(1)),
            SimplifiedNode::Multiply(vec![
                SimplifiedNode::Number(rat!(-1)),
                SimplifiedNode::Number(rat!(5)),
            ])
        ])
    );
}

#[test]
fn test_reduction() {
    assert_eq!(
        reduce!(simplify!(tokens!(1 2 + 5 6 + 3 4))),
        SimplifiedNode::Number(rat!(102))
    );

    assert_eq!(
        reduce!(simplify!(tokens!(1 2 + 5 6 * 2 + 3 4))),
        SimplifiedNode::Number(rat!(158))
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            token!(1),
            token!(+),
            UnstructuredNode::Parentheses(uns_list!(
                token!(2),
                token!(+),
                UnstructuredNode::Parentheses(uns_list!(
                    token!(3),
                    token!(*),
                    UnstructuredNode::Parentheses(uns_list!(
                        token!(4),
                        token!(*),
                        token!(5),
                    )),
                    token!(*),
                    token!(6),
                )),
                token!(+),
                UnstructuredNode::Parentheses(uns_list!(
                    token!(7),
                    token!(+),
                    token!(8),
                )),
            )),
            token!(*),
            token!(9),
        ))),
        SimplifiedNode::Number(rat!(3394))
    );

    assert_eq!(
        reduce!(simplify!(tokens!(8 / 2))),
        SimplifiedNode::Number(rat!(4))
    );

    assert_eq!(
        reduce!(simplify!(tokens!(3 / 5 + 2 / 3 + 7 / 4))),
        SimplifiedNode::Number(rat!(181, 60))
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            UnstructuredNode::Fraction(
                tokens!(2),
                tokens!(3),
            )
        ))),
        SimplifiedNode::Number(rat!(2, 3))
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            token!(5),
            UnstructuredNode::Power(uns_list!(
                UnstructuredNode::Fraction(
                    tokens!(2),
                    tokens!(3),
                )
            ))
        ))),
        SimplifiedNode::Power(
            box SimplifiedNode::Number(rat!(25)),
            box SimplifiedNode::Number(rat!(1, 3)),
        )
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            token!(var x),
            token!(*),
            token!(var x),
            UnstructuredNode::Power(tokens!(3)),
            token!(*),
            token!(var y),
            UnstructuredNode::Power(tokens!(10)),
            token!(*),
            token!(var x),
            token!(*),
            token!(var y),
        ))),
        SimplifiedNode::Multiply(vec![
            SimplifiedNode::Power(
                box SimplifiedNode::Variable('x'),
                box SimplifiedNode::Number(rat!(5)),
            ),
            SimplifiedNode::Power(
                box SimplifiedNode::Variable('y'),
                box SimplifiedNode::Number(rat!(11)),
            ),
        ])
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            UnstructuredNode::Parentheses(uns_list!(
                token!(var x),
                token!(var x),
                token!(var y),
            )),
            UnstructuredNode::Power(tokens!(3)),
        ))),
        SimplifiedNode::Multiply(vec![
            SimplifiedNode::Power(
                box SimplifiedNode::Variable('x'),
                box SimplifiedNode::Number(rat!(6)),
            ),
            SimplifiedNode::Power(
                box SimplifiedNode::Variable('y'),
                box SimplifiedNode::Number(rat!(3)),
            ),
        ])
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            token!(3),
            token!(var x),
            UnstructuredNode::Power(tokens!(2)),
        ))),
        SimplifiedNode::Multiply(vec![
            SimplifiedNode::Number(rat!(3)),
            SimplifiedNode::Power(
                box SimplifiedNode::Variable('x'),
                box SimplifiedNode::Number(rat!(2)),
            ),
        ]),
    );

    assert_eq!(
        reduce!(simplify!(uns_list!(
            token!(2),
            token!(var x),
            token!(+),
            token!(3),
            token!(var x),
            UnstructuredNode::Power(tokens!(2)),
            token!(+),
            token!(6),
            token!(var x),
            UnstructuredNode::Power(tokens!(2)),
            token!(+),
            token!(var x),
            token!(+),
            token!(7),
        ))),
        SimplifiedNode::Add(vec![
            SimplifiedNode::Number(rat!(7)),
            SimplifiedNode::Multiply(vec![
                SimplifiedNode::Number(rat!(3)),
                SimplifiedNode::Variable('x'),
            ]),
            SimplifiedNode::Multiply(vec![
                SimplifiedNode::Number(rat!(9)),
                SimplifiedNode::Power(
                    box SimplifiedNode::Variable('x'),
                    box SimplifiedNode::Number(rat!(2)),
                ),
            ]),
        ])
    );
}

#[bench]
fn bench_unstructured_layout(b: &mut Bencher) {
    let tree = complex_unstructured_expression();
    let mut ascii_renderer = AsciiRenderer::default();

    b.iter(|| {
        black_box(tree.layout(&mut ascii_renderer, None, LayoutComputationProperties::default()));
    });
}

#[test]
fn test_divide_by_zero() {
    // Rational
    let result = StructuredNode::Divide(
        box StructuredNode::Number(rat!(12)),
        box StructuredNode::Number(rat!(0)),
    ).disambiguate().unwrap().evaluate(&EvaluationSettings::default());
    assert_matches!(result, Err(_));

    // Decimal
    let result = StructuredNode::Add(
        box StructuredNode::Divide(
            box StructuredNode::Number(dec!(12)),
            box StructuredNode::Number(dec!(0)),
        ),
        box StructuredNode::Number(dec!(0.1)),
    ).disambiguate().unwrap().evaluate(&EvaluationSettings::default());
    assert_matches!(result, Err(_));
}

#[test]
fn test_size_reduction_level() {
    let block = UnstructuredNodeRoot { root: uns_list!(
        token!(1),
        token!(+),
        token!(2),
        UnstructuredNode::Power(tokens!(3)),
    ) }.upgrade().unwrap().layout(&mut AsciiRenderer::default(), None, LayoutComputationProperties::default());

    // The "3" token should have a size reduction level of 1, all others should have 0
    for (glyph, _) in block.glyphs {
        if let Glyph::Digit { number: 3 } = glyph.glyph {
            assert_eq!(glyph.size_reduction_level, 1);
        } else {
            assert_eq!(glyph.size_reduction_level, 0);
        }
    }
}

#[test]
fn test_function_evaluation() {
    assert_eq!(
        Function::Sine.evaluate(&[dec!(90)], &EvaluationSettings { angle_unit: AngleUnit::Degree, use_floats: false }),
        Ok(dec_approx!(1)),
    );
    assert_eq!(
        Function::Sine.evaluate(&[Number::Decimal(Decimal::PI / Decimal::TWO, DecimalAccuracy::Exact)], &EvaluationSettings { angle_unit: AngleUnit::Radian, use_floats: false }),
        Ok(dec_approx!(1)),
    );

    assert_eq!(
        Function::Cosine.evaluate(&[dec!(180)], &EvaluationSettings { angle_unit: AngleUnit::Degree, use_floats: false }),
        Ok(dec_approx!(-1)),
    );
    assert_eq!(
        Function::Cosine.evaluate(&[Number::Decimal(Decimal::PI, DecimalAccuracy::Exact)], &EvaluationSettings { angle_unit: AngleUnit::Radian, use_floats: false }),
        Ok(dec_approx!(-1)),
    );
}

#[test]
fn test_correct_float() {
    // Down
    assert_eq!(dec_approx!(2.0000000000000000).correct_inaccuracy(), dec_approx!(2));
    assert_eq!(dec_approx!(2.0000000000000001).correct_inaccuracy(), dec_approx!(2));
    assert_eq!(dec_approx!(5.1402000000000000).correct_inaccuracy(), dec_approx!(5.1402));
    assert_eq!(dec_approx!(5.1402000000000001).correct_inaccuracy(), dec_approx!(5.1402));

    // Up
    assert_eq!(dec_approx!(1.9999999999999999).correct_inaccuracy(), dec_approx!(2));
    assert_eq!(dec_approx!(1.9999999999999997).correct_inaccuracy(), dec_approx!(2));
    assert_eq!(dec_approx!(5.1401999999999999).correct_inaccuracy(), dec_approx!(5.1402));
    assert_eq!(dec_approx!(5.1401999999999997).correct_inaccuracy(), dec_approx!(5.1402));

    // Zero and negatives
    assert_eq!(dec_approx!(0.0000000000000001).correct_inaccuracy(), dec_approx!(0));
    assert_eq!(dec_approx!(-4.999999999999997).correct_inaccuracy(), dec_approx!(-5));
    assert_eq!(dec_approx!(-5.000000000000001).correct_inaccuracy(), dec_approx!(-5));
    assert_eq!(dec_approx!(-4.130000000000001).correct_inaccuracy(), dec_approx!(-4.13));
    assert_eq!(dec_approx!(-4.129999999999999).correct_inaccuracy(), dec_approx!(-4.13));

    // Classic real-world case
    let es = EvaluationSettings::default();
    assert_eq!(
        (
            Function::Sine.evaluate(&[dec!(1)], &es).unwrap().powi(2)
            + Function::Cosine.evaluate(&[dec!(1)], &es).unwrap().powi(2)
        ).correct_inaccuracy(),
        dec_approx!(1),
    );
}
