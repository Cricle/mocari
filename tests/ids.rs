use rusty_live2d::{DrawableId, Error, Id, ParameterId, PartId};

#[test]
fn id_rejects_empty_strings() {
    let error = Id::new("").unwrap_err();
    assert!(matches!(error, Error::EmptyId));
}

#[test]
fn id_trims_nothing_and_preserves_source_text() {
    let id = Id::new(" ParamAngleX ").unwrap();
    assert_eq!(id.as_str(), " ParamAngleX ");
    assert_eq!(id.to_string(), " ParamAngleX ");
}

#[test]
fn typed_ids_expose_their_inner_text() {
    let parameter = ParameterId::new("ParamAngleX").unwrap();
    let part = PartId::new("PartArmL").unwrap();
    let drawable = DrawableId::new("DrawableBody").unwrap();

    assert_eq!(parameter.as_str(), "ParamAngleX");
    assert_eq!(part.as_str(), "PartArmL");
    assert_eq!(drawable.as_str(), "DrawableBody");
}
