pub use glam::{Mat4, Vec2};

pub trait Mat4Ext {
    fn scale_x(&self) -> f32;
    fn scale_y(&self) -> f32;
    fn transform_x(&self, value: f32) -> f32;
    fn transform_y(&self, value: f32) -> f32;
    fn invert_transform_x(&self, value: f32) -> f32;
    fn invert_transform_y(&self, value: f32) -> f32;
}

impl Mat4Ext for Mat4 {
    fn scale_x(&self) -> f32 {
        self.x_axis.x
    }

    fn scale_y(&self) -> f32 {
        self.y_axis.y
    }

    fn transform_x(&self, value: f32) -> f32 {
        self.x_axis.x * value + self.w_axis.x
    }

    fn transform_y(&self, value: f32) -> f32 {
        self.y_axis.y * value + self.w_axis.y
    }

    fn invert_transform_x(&self, value: f32) -> f32 {
        (value - self.w_axis.x) / self.x_axis.x
    }

    fn invert_transform_y(&self, value: f32) -> f32 {
        (value - self.w_axis.y) / self.y_axis.y
    }
}

pub fn direction_to_radian(from: Vec2, to: Vec2) -> f32 {
    let mut result = to.y.atan2(to.x) - from.y.atan2(from.x);

    while result < -std::f32::consts::PI {
        result += std::f32::consts::PI * 2.0;
    }

    while result > std::f32::consts::PI {
        result -= std::f32::consts::PI * 2.0;
    }

    result
}

pub fn radian_to_direction(radian: f32) -> Vec2 {
    Vec2::new(radian.sin(), radian.cos())
}
