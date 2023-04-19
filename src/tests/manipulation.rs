use alloc::boxed::Box;

use crate::{StructuredNode, node::{structured::EvaluationSettings, unstructured::Upgradable}, UnstructuredNode, UnstructuredNodeRoot, tests::util::complex_unstructured_expression};

#[test]
fn test_disambiguate() {
    let tree = StructuredNode::Multiply(
        Box::new(StructuredNode::Number(1.into())),
        Box::new(StructuredNode::Multiply(
            Box::new(StructuredNode::Number(2.into())),
            Box::new(StructuredNode::Number(3.into())),
        )),
    );
    assert_eq!(
        tree.disambiguate().unwrap(),
        StructuredNode::Multiply(
            Box::new(StructuredNode::Number(1.into())),
            Box::new(StructuredNode::Parentheses(
                Box::new(StructuredNode::Multiply(
                    Box::new(StructuredNode::Number(2.into())),
                    Box::new(StructuredNode::Number(3.into())),
                )),
            )),
        )
    );

    let tree = StructuredNode::Multiply(
        Box::new(StructuredNode::Add(
            Box::new(StructuredNode::Number(1.into())),
            Box::new(StructuredNode::Number(2.into())),
        )),
        Box::new(StructuredNode::Add(
            Box::new(StructuredNode::Number(3.into())),
            Box::new(StructuredNode::Number(4.into())),
        )),
    );
    assert_eq!(
        tree.disambiguate().unwrap(),
        StructuredNode::Multiply(
            Box::new(StructuredNode::Parentheses(
                Box::new(StructuredNode::Add(
                    Box::new(StructuredNode::Number(1.into())),
                    Box::new(StructuredNode::Number(2.into())),
                )),
            )),
            Box::new(StructuredNode::Parentheses(
                Box::new(StructuredNode::Add(
                    Box::new(StructuredNode::Number(3.into())),
                    Box::new(StructuredNode::Number(4.into())),
                )),
            )),
        )
    );
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
