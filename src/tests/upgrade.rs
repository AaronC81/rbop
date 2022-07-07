use crate::{node::{unstructured::Upgradable, structured::EvaluationSettings}, StructuredNode, renderers::AsciiRenderer, render::Renderer, UnstructuredNode};

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
