use std::fmt;

/// Errors from AI rigging operations.
#[derive(Debug)]
pub enum RigError {
    /// Input image/PSD could not be decoded.
    InvalidInput(String),
    /// AI inference failed.
    InferenceFailed(String),
    /// Output data is invalid or incomplete.
    InvalidOutput(String),
}

impl fmt::Display for RigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::InferenceFailed(msg) => write!(f, "inference failed: {msg}"),
            Self::InvalidOutput(msg) => write!(f, "invalid output: {msg}"),
        }
    }
}

impl std::error::Error for RigError {}
