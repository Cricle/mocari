use rusty_live2d::json::{
    Expression3, ExpressionBlend, apply_expression_blend, apply_expression_parameter,
};

#[test]
fn parses_expression3_parameters() {
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "FadeInTime": 0.5,
            "FadeOutTime": 0.25,
            "Parameters": [
                { "Id": "ParamEyeLOpen", "Value": 0.0, "Blend": "Overwrite" },
                { "Id": "ParamMouthOpenY", "Value": 0.7, "Blend": "Add" },
                { "Id": "ParamCheek", "Value": 0.5, "Blend": "Multiply" }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(expression.kind(), "Live2D Expression");
    assert_eq!(expression.fade_in_time(), Some(0.5));
    assert_eq!(expression.fade_out_time(), Some(0.25));
    assert_eq!(expression.parameters()[0].id(), "ParamEyeLOpen");
    assert_eq!(expression.parameters()[0].value(), 0.0);
    assert_eq!(
        expression.parameters()[0].blend(),
        ExpressionBlend::Overwrite
    );
    assert_eq!(expression.parameters()[1].blend(), ExpressionBlend::Add);
    assert_eq!(
        expression.parameters()[2].blend(),
        ExpressionBlend::Multiply
    );
}

#[test]
fn expression3_defaults_to_add_blend() {
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamEyeLOpen", "Value": 1.0 }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(expression.parameters()[0].blend(), ExpressionBlend::Add);
}

#[test]
fn expression3_unknown_blend_falls_back_to_add() {
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamEyeLOpen", "Value": 1.0, "Blend": "Screen" }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(expression.parameters()[0].blend(), ExpressionBlend::Add);
}

#[test]
fn applies_expression_blend_modes() {
    assert_eq!(
        apply_expression_blend(0.25, 0.5, ExpressionBlend::Add, 0.4),
        0.45
    );
    assert_eq!(
        apply_expression_blend(2.0, 1.5, ExpressionBlend::Multiply, 0.25),
        2.25
    );
    assert_eq!(
        apply_expression_blend(0.25, 0.75, ExpressionBlend::Overwrite, 0.5),
        0.5
    );
}

#[test]
fn applies_expression_parameter() {
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamMouthOpenY", "Value": 0.7, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(
        apply_expression_parameter(0.2, &expression.parameters()[0], 0.5),
        0.55
    );
}
