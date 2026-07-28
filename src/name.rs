//! Exact spelling data stored by one module table.

/// One exact, case-sensitive spelling.
///
/// A name table performs no casing, normalization, derivation, or semantic
/// interpretation. Those operations belong to textual projection.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct Name(String);

impl Name {
    /// Construct an exact spelling.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The exact spelling as UTF-8 text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The exact UTF-8 bytes used by canonical allocation ordering.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value)
    }
}
