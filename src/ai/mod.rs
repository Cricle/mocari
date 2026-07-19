//! AI integration traits and types.
//!
//! This module defines traits for AI-assisted model rigging (`AiRigger`)
//! and AI-driven runtime character control (`AiDriver`). mocari provides
//! no implementations — users plug in their preferred AI backend.

mod driver;
mod error;
mod rigger;

pub use driver::AiDriver;
pub use error::RigError;
pub use rigger::{
    AiRigger, DeformerChild, DeformerType, InterpolationType, ParameterKeyframe,
    RiggedDeformer, RiggedMesh, RiggedModel, RiggedParameter,
};
