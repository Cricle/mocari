//! Shared parser and runtime error types.
//!
//! Higher-level loaders wrap this error with path context. Lower-level parsing
//! APIs return it directly when model data is malformed or uses an unsupported
//! format version.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Error type used by Mocari parsers and mesh-building helpers.
pub enum Error {
    /// An id string was empty where Cubism data requires a named item.
    EmptyId,
    /// A Cubism JSON sidecar file was malformed.
    InvalidJson {
        /// Human-readable format name, such as `model3.json`.
        format: &'static str,
        /// Specific validation failure.
        message: String,
    },
    /// A `.moc3` file was malformed or internally inconsistent.
    InvalidMoc3 {
        /// Specific validation failure.
        message: String,
    },
    /// The file version is known but not supported by this crate.
    UnsupportedVersion {
        /// Human-readable format name.
        format: &'static str,
        /// Version number read from the file.
        version: u32,
    },
}

/// Result alias used by lower-level Mocari APIs.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("id cannot be empty"),
            Self::InvalidJson { format, message } => {
                write!(formatter, "invalid {format}: {message}")
            }
            Self::InvalidMoc3 { message } => write!(formatter, "invalid moc3: {message}"),
            Self::UnsupportedVersion { format, version } => {
                write!(formatter, "unsupported {format} version {version}")
            }
        }
    }
}

impl std::error::Error for Error {}
