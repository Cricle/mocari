use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptyId,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("id cannot be empty"),
        }
    }
}

impl std::error::Error for Error {}
