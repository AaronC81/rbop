use alloc::vec;

use crate::{node::simplified::SimplifiedNode, UnstructuredNode};

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
