pub use glam::{Mat4, Vec2};

pub trait Mat4Ext {
    fn scale_x(&self) -> f32;
    fn scale_y(&self) -> f32;
    fn transform_x(&self, value: f32) -> f32;
    fn transform_y(&self, value: f32) -> f32;
    fn invert_transform_x(&self, value: f32) -> f32;
    fn invert_transform_y(&self, value: f32) -> f32;
    fn set_scale(&mut self, sx: f32, sy: f32);
    fn set_translation(&mut self, tx: f32, ty: f32);
    fn scale(&mut self, sx: f32, sy: f32);
    fn translate(&mut self, tx: f32, ty: f32);
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

    fn set_scale(&mut self, sx: f32, sy: f32) {
        self.x_axis.x = sx;
        self.y_axis.y = sy;
    }

    fn set_translation(&mut self, tx: f32, ty: f32) {
        self.w_axis.x = tx;
        self.w_axis.y = ty;
    }

    fn scale(&mut self, sx: f32, sy: f32) {
        self.x_axis.x = sx;
        self.y_axis.y = sy;
    }

    fn translate(&mut self, tx: f32, ty: f32) {
        self.w_axis.x = tx;
        self.w_axis.y = ty;
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
