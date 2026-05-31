use super::math::{Vector2, degrees_to_radian};

pub fn rotation_deformer_transform_point(
    point: Vector2,
    angle_degrees: f32,
    scale: f32,
    translation: Vector2,
    flip_x: bool,
    flip_y: bool,
) -> Vector2 {
    let theta = degrees_to_radian(angle_degrees);
    let cos = theta.cos();
    let sin = theta.sin();
    let sign_x = if flip_x { -1.0 } else { 1.0 };
    let sign_y = if flip_y { -1.0 } else { 1.0 };

    let m00 = cos * scale * sign_x;
    let m01 = -sin * scale * sign_y;
    let m10 = sin * scale * sign_x;
    let m11 = cos * scale * sign_y;

    Vector2::new(
        m00 * point.x() + m01 * point.y() + translation.x(),
        m10 * point.x() + m11 * point.y() + translation.y(),
    )
}
