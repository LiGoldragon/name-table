//! The crate-boundary error type.

use thiserror::Error;

use crate::identifier::Identifier;

/// A failure crossing the name-table boundary. Rendering carries the underlying
/// rkyv message (as text) so callers see the concrete cause without depending on
/// rkyv's own error type at this crate's edge.
#[derive(Debug, Clone, Error)]
pub enum NameTableError {
    /// A resolve was asked for an identifier the table never interned.
    #[error("no name interned for {0}")]
    UnknownIdentifier(Identifier),

    /// Serializing the table's canonical name bytes failed.
    #[error("name-table serialization failed: {0}")]
    Serialize(String),

    /// Deserializing (with validation-on-read) the table's name bytes failed.
    #[error("name-table deserialization failed: {0}")]
    Deserialize(String),
}
