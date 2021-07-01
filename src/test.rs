use crate::{Node, Token};

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
