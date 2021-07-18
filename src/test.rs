use crate::nav::NavPath;
use crate::{Node, Token, render::Renderer};
use crate::renderers::AsciiRenderer;

#[test]
fn test_upgrade() {
    let unstructured = Node::Unstructured(vec![
        Node::Token(Token::Digit(1)),
        Node::Token(Token::Digit(2)),
        Node::Token(Token::Multiply),
        Node::Token(Token::Digit(3)),
        Node::Token(Token::Digit(4)),
        Node::Token(Token::Add),
        Node::Token(Token::Digit(5)),
        Node::Token(Token::Digit(6)),
        Node::Token(Token::Multiply),
        Node::Token(Token::Digit(7)),
        Node::Token(Token::Digit(8)),
    ]);

    assert_eq!(
        unstructured.upgrade().unwrap(),
        Node::Add(
            box Node::Multiply(
                box Node::Number(12),
                box Node::Number(34),
            ),
            box Node::Multiply(
                box Node::Number(56),
                box Node::Number(78),
            ),
        )
    );
}

#[test]
fn test_disambiguate() {
    let tree = Node::Multiply(
        box Node::Number(1),
        box Node::Multiply(
            box Node::Number(2),
            box Node::Number(3),
        ),
    );
    assert_eq!(
        tree.disambiguate().unwrap(),
        Node::Multiply(
            box Node::Number(1),
            box Node::Parentheses(
                box Node::Multiply(
                    box Node::Number(2),
                    box Node::Number(3),
                ),
            ),
        )
    );

    let tree = Node::Multiply(
        box Node::Add(
            box Node::Number(1),
            box Node::Number(2),
        ),
        box Node::Add(
            box Node::Number(3),
            box Node::Number(4),
        ),
    );
    assert_eq!(
        tree.disambiguate().unwrap(),
        Node::Multiply(
            box Node::Parentheses(
                box Node::Add(
                    box Node::Number(1),
                    box Node::Number(2),
                ),
            ),
            box Node::Parentheses(
                box Node::Add(
                    box Node::Number(3),
                    box Node::Number(4),
                ),
            ),
        )
    );
}

#[test]
fn test_ascii_render() {
    let tree = Node::Add(
        box Node::Multiply(
            box Node::Number(12),
            box Node::Number(34),
        ),
        box Node::Multiply(
            box Node::Number(56),
            box Node::Number(78),
        ),
    ).disambiguate().unwrap();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(tree);
    assert_eq!(
        renderer.lines,
        vec!["12*34+56*78"],
    );

    let tree = Node::Add(
        box Node::Add(
            box Node::Number(12),
            box Node::Divide(
                box Node::Add(
                    box Node::Number(34),
                    box Node::Divide(
                        box Node::Number(56),
                        box Node::Number(78),
                    )
                ),
                box Node::Number(90),
            ),
        ),
        box Node::Number(12),
    ).disambiguate().unwrap();
    let mut renderer = AsciiRenderer::default();
    renderer.draw_all(tree);
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
}

#[test]
fn test_navigation() {
    let mut div_bot = box Node::Unstructured(vec![
        Node::Token(Token::Digit(7)),
        Node::Token(Token::Digit(8)),
    ]);
    let div_bot_ptr = &mut *div_bot as *mut Node;
    let mut unstructured = Node::Unstructured(vec![
        Node::Token(Token::Digit(1)),
        Node::Token(Token::Digit(2)),
        Node::Token(Token::Multiply),
        Node::Token(Token::Digit(3)),
        Node::Token(Token::Digit(4)),
        Node::Token(Token::Add),
        Node::Divide(
            box Node::Unstructured(vec![
                Node::Token(Token::Digit(5)),
                Node::Token(Token::Digit(6)),
            ]),
            div_bot,
        ),
    ]);

    // Path 1: beginning
    let mut path = NavPath::new(vec![0]);
    let result = {
        let (node, i) = unstructured.navigate_mut(&mut path.to_navigator());
        let node_ptr: *mut Node = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (&mut unstructured as *mut Node, 0)
    );

    // Path 2: middle
    let mut path = NavPath::new(vec![3]);
    let result = {
        let (node, i) = unstructured.navigate_mut(&mut path.to_navigator());
        let node_ptr: *mut Node = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (&mut unstructured as *mut Node, 3)
    );

    // Path 3: nested
    let mut path = NavPath::new(vec![6, 1, 1]);
    let result = {
        let (node, i) = unstructured.navigate_mut(&mut path.to_navigator());
        let node_ptr: *mut Node = node;
        (node_ptr, i)
    };
    assert_eq!(
        result,
        (div_bot_ptr, 1)
    );
}
