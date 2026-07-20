#![forbid(unsafe_code)]
//! A pure Rust runtime for reading and driving Live2D/Cubism-compatible model data.
//!
//! Mocari is split into a small set of layers so applications can choose how much
//! control they need:
//!
//! - [`assets`] loads a `.model3.json` file, its referenced `.moc3` data, pose
//!   file, and PNG textures from disk.
//! - [`ModelRuntime`] owns the mutable model state used by motions, expressions,
//!   pose fading, and drawable mesh generation.
//! - [`motion`] and [`expression`] provide lightweight players for Cubism motion
//!   and expression JSON files.
//! - [`render::common`] contains backend-neutral draw ordering, clipping, and
//!   vertex helpers for custom renderers.
//! - `render::wgpu` provides a ready-to-use renderer when the `wgpu` feature is
//!   enabled.
//!
//! The usual flow is to load a runtime model, update its parameters each frame,
//! apply motions or expressions, rebuild meshes, then pass those meshes to a
//! renderer.
//!
//! ```no_run
//! use mocari::{assets::load_model_runtime, MotionPlayer};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json")?;
//! let runtime = model.runtime_mut();
//!
//! runtime.set_parameter_normalized("ParamAngleX", 0.75);
//! runtime.update_meshes();
//!
//! for mesh in runtime.meshes() {
//!     // Upload mesh.vertices() and mesh.indices() to your renderer.
//! }
//! # Ok(())
//! # }
//! ```

/// Filesystem helpers for loading model assets and decoded textures.
pub mod assets;
/// Buffer pooling for reducing allocations during mesh updates.
pub mod buffer_pool;
/// Math, interpolation, parameter, and physics primitives used by the runtime.
pub mod core;
/// Shared error types returned by parsers and lower-level runtime code.
pub mod error;
/// Expression playback and blending against a [`ModelRuntime`].
pub mod expression;
/// Parsers and data models for Cubism JSON sidecar files.
pub mod json;
/// Parsers and mesh builders for `.moc3` model data.
pub mod moc3;
/// Motion playback against a [`ModelRuntime`].
pub mod motion;
/// Renderer-facing helpers and optional backend implementations.
pub mod render;
/// Auto-animation features for eye blink, lip sync, breath, and mouse tracking.
pub mod auto;
/// High-level engine that encapsulates wgpu setup, rendering, and animation.
#[cfg(feature = "wgpu")]
pub mod engine;
/// Convenience re-exports for the high-level engine API.
#[cfg(feature = "wgpu")]
pub mod prelude {
    pub use crate::engine::{EngineError, Live2dEngine, ModelHandle};
    pub use crate::engine::{FrameContext, Live2dPlugin, RenderContext};
}
/// Mutable model state used for parameter edits, pose updates, and mesh output.
pub mod runtime;
/// MCP (Model Context Protocol) server for controlling Live2D models.
#[cfg(feature = "mcp")]
pub mod mcp;
/// AI integration traits for rigging and runtime character control.
#[cfg(feature = "ai")]
pub mod ai;

pub use crate::auto::{Breath, BreathConfig, EyeBlink, EyeBlinkConfig, LipSync, LipSyncConfig, MouseTracker, MouseTrackerConfig};
pub use crate::error::{Error, Result};
pub use crate::expression::{ExpressionManager, ExpressionPlayer};
pub use crate::json::{ExpressionTarget, MotionUserData, UserData3};
pub use crate::motion::{MotionManager, MotionPlayer, MotionPriority};
pub use crate::runtime::{HitAreaInfo, ModelRuntime, ParameterInfo};
