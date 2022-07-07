use alloc::vec;

use crate::{StructuredNode, node::{unstructured::Upgradable, structured::EvaluationSettings, function::Function}, error::NodeError};

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
