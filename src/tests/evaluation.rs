use core::assert_matches::assert_matches;

use alloc::boxed::Box;
use rust_decimal::Decimal;

use crate::{StructuredNode, node::{structured::{EvaluationSettings, AngleUnit}, function::Function}, Number, number::DecimalAccuracy};


#[test]
fn test_divide_by_zero() {
    // Rational
    let result = StructuredNode::Divide(
        Box::new(StructuredNode::Number(rat!(12))),
        Box::new(StructuredNode::Number(rat!(0))),
    ).disambiguate().unwrap().evaluate(&EvaluationSettings::default());
    assert_matches!(result, Err(_));

    // Decimal
    let result = StructuredNode::Add(
        Box::new(StructuredNode::Divide(
            Box::new(StructuredNode::Number(dec!(12))),
            Box::new(StructuredNode::Number(dec!(0))),
        )),
        Box::new(StructuredNode::Number(dec!(0.1))),
    ).disambiguate().unwrap().evaluate(&EvaluationSettings::default());
    assert_matches!(result, Err(_));
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
