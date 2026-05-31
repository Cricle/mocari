use rusty_live2d::{
    Error,
    json::{Expression3, ExpressionBlend},
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
fn expression3_defaults_to_overwrite_blend() {
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamEyeLOpen", "Value": 1.0 }
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(
        expression.parameters()[0].blend(),
        ExpressionBlend::Overwrite
    );
}

#[test]
fn rejects_unknown_expression3_blend() {
    let error = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamEyeLOpen", "Value": 1.0, "Blend": "Screen" }
            ]
        }"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidJson {
            format: "exp3.json",
            ..
        }
    ));
}
