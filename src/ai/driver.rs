use crate::runtime::ModelRuntime;

/// A per-frame AI driver that injects parameter changes.
///
/// Multiple drivers can be registered simultaneously.
/// They run in registration order, after model tick.
#[cfg(not(target_arch = "wasm32"))]
pub trait AiDriver: Send + Sync {
    /// Called every frame after model tick.
    /// Use `model.set_parameter()` to drive the character.
    fn update(&mut self, delta: f32, model: &mut ModelRuntime);
}

/// A per-frame AI driver that injects parameter changes.
///
/// Multiple drivers can be registered simultaneously.
/// They run in registration order, after model tick.
#[cfg(target_arch = "wasm32")]
pub trait AiDriver {
    /// Called every frame after model tick.
    /// Use `model.set_parameter()` to drive the character.
    fn update(&mut self, delta: f32, model: &mut ModelRuntime);
}
