use crate::{node::{unstructured::Upgradable, structured::EvaluationSettings}, StructuredNode, renderers::AsciiRenderer, render::Renderer, UnstructuredNode};

#[test]
fn test_upgrade() {
    let unstructured = tokens!(1 2 * 3 4 + 5 6 * 7 8);
    
    assert_eq!(
        unstructured.upgrade().unwrap(),
        StructuredNode::Add(
            Box::new(StructuredNode::Multiply(
                Box::new(StructuredNode::Number(12.into())),
                Box::new(StructuredNode::Number(34.into())),
            )),
            Box::new(StructuredNode::Multiply(
                Box::new(StructuredNode::Number(56.into())),
                Box::new(StructuredNode::Number(78.into())),
            )),
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
fn test_implicit_multiply() {
    // 2x = 2 * x
    assert_eq!(
        uns_list!(
            token!(2),
            token!(var x),
        ).upgrade().unwrap(),
        StructuredNode::Multiply(
            Box::new(StructuredNode::Number(rat!(2))),
            Box::new(StructuredNode::Variable('x')),
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
            Box::new(StructuredNode::Number(dec!(0.5))),
            Box::new(StructuredNode::Variable('x')),
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
            Box::new(StructuredNode::Number(rat!(2))),
            Box::new(StructuredNode::Parentheses(
                Box::new(StructuredNode::Add(
                    Box::new(StructuredNode::Number(rat!(1))),
                    Box::new(StructuredNode::Variable('x')),
                ))
            ))
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
            Box::new(StructuredNode::Multiply(
                Box::new(StructuredNode::Variable('x')),
                Box::new(StructuredNode::Multiply(
                    Box::new(StructuredNode::Variable('y')),
                    Box::new(StructuredNode::Variable('z')),
                )),
            )),
            Box::new(StructuredNode::Number(rat!(2))),
        )
    );
}
