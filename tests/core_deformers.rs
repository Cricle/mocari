use rusty_live2d::core::{Vector2, rotation_deformer_transform_point};

fn assert_vec_close(actual: Vector2, expected: Vector2) {
    assert!(
        (actual.x() - expected.x()).abs() < 0.00001,
        "expected x {}, got {}",
        expected.x(),
        actual.x()
    );
    assert!(
        (actual.y() - expected.y()).abs() < 0.00001,
        "expected y {}, got {}",
        expected.y(),
        actual.y()
    );
}

#[test]
fn rotation_deformer_applies_confirmed_forward_matrix() {
    let point = Vector2::new(1.0, 2.0);

    let transformed =
        rotation_deformer_transform_point(point, 90.0, 2.0, Vector2::new(10.0, 20.0), false, false);

    assert_vec_close(transformed, Vector2::new(6.0, 22.0));
}

#[test]
fn rotation_deformer_applies_flip_signs_to_axes() {
    let point = Vector2::new(1.0, 2.0);

    let transformed =
        rotation_deformer_transform_point(point, 90.0, 2.0, Vector2::new(10.0, 20.0), true, true);

    assert_vec_close(transformed, Vector2::new(14.0, 18.0));
}
