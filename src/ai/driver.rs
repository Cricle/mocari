use crate::runtime::ModelRuntime;

/// A per-frame AI driver that injects parameter changes.
///
/// Multiple drivers can be registered simultaneously.
/// They run in registration order, before the engine's tick.
pub trait AiDriver: Send + Sync {
    /// Called every frame before engine tick.
    /// Use `model.set_parameter()` to drive the character.
    fn update(&mut self, delta: f32, model: &mut ModelRuntime);
}
