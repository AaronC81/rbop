use alloc::vec;

use crate::{StructuredNode, tests::util::complex_unstructured_expression, nav::NavPath, render::{Viewport, Area, CalculatedPoint, Layoutable, LayoutComputationProperties, Glyph}, UnstructuredNode, node::{structured::EvaluationSettings, unstructured::Upgradable}, UnstructuredNodeRoot, Number, renderers::AsciiRenderer};

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
fn test_power_with_no_base() {
    // This used to panic - make sure it doesn't
    let block = UnstructuredNodeRoot { root: uns_list!(UnstructuredNode::Power(uns_list!())) };
    block.layout(&mut AsciiRenderer::default(), None, LayoutComputationProperties::default());
}
