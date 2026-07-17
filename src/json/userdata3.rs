use serde::Deserialize;

use crate::{Error, Result};

const FORMAT: &str = "userdata3.json";

#[derive(Debug, Clone, PartialEq, Eq)]
/// Target type for a user data entry.
pub enum UserDataTarget {
    /// User data attached to a parameter.
    Parameter,
    /// User data attached to a part.
    Part,
    /// User data attached to a drawable.
    Drawable,
}

impl UserDataTarget {
    fn from_raw(raw: &str) -> Self {
        match raw {
            "Part" => Self::Part,
            "Drawable" => Self::Drawable,
            _ => Self::Parameter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One entry in a `userdata3.json` file.
pub struct UserDataEntry {
    target: UserDataTarget,
    id: String,
    value: String,
}

impl UserDataEntry {
    /// Returns the target type.
    pub fn target(&self) -> &UserDataTarget {
        &self.target
    }

    /// Returns the target element id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the user data string value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed Cubism `userdata3.json` data.
pub struct UserData3 {
    version: u32,
    entries: Vec<UserDataEntry>,
}

impl UserData3 {
    /// Parses a `userdata3.json` document from a string.
    pub fn from_json_str(source: &str) -> Result<Self> {
        let raw: RawUserData3 =
            serde_json::from_str(source).map_err(|error| Error::InvalidJson {
                format: FORMAT,
                message: error.to_string(),
            })?;

        Ok(Self {
            version: raw.version,
            entries: raw
                .user_data
                .into_iter()
                .map(|raw| UserDataEntry {
                    target: UserDataTarget::from_raw(&raw.target),
                    id: raw.id,
                    value: raw.value,
                })
                .collect(),
        })
    }

    /// Returns the format version.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns all user data entries.
    pub fn entries(&self) -> &[UserDataEntry] {
        &self.entries
    }

    /// Finds the first entry matching the given target and id.
    pub fn find(&self, target: &UserDataTarget, id: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.target == *target && entry.id == id)
            .map(|entry| entry.value.as_str())
    }
}

#[derive(Debug, Deserialize)]
struct RawUserData3 {
    #[serde(rename = "Version")]
    version: u32,
    #[serde(rename = "UserData")]
    user_data: Vec<RawUserDataEntry>,
}

#[derive(Debug, Deserialize)]
struct RawUserDataEntry {
    #[serde(rename = "Target")]
    target: String,
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Value")]
    value: String,
}
