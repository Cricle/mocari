#![forbid(unsafe_code)]

pub mod core;
pub mod error;

pub use crate::core::{DrawableId, Id, ParameterId, PartId};
pub use crate::error::{Error, Result};
