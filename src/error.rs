//! The crate-boundary error type.

use thiserror::Error;

use crate::identifier::{Identifier, IdentifierNamespace};
use crate::name::Name;

/// A failure crossing the name-table boundary.
#[derive(Debug, Clone, Error)]
pub enum NameTableError {
    /// A resolve was asked for an identifier the table never interned.
    #[error("no name interned for {0}")]
    UnknownIdentifier(Identifier),

    /// A composed table has no slice for the requested identifier namespace.
    #[error("the composed NameTable does not borrow {0:?}")]
    UnknownNamespace(IdentifierNamespace),

    /// Composition attempted to add a namespace already represented in the
    /// component's single composed NameTable.
    #[error("the composed NameTable already represents {0:?}")]
    DuplicateNamespace(IdentifierNamespace),

    /// The home slice is already borrowed by a composed consumer and is sealed
    /// against later mutation. Callers must complete allocation before composition.
    #[error("cannot {operation}: this NameTable home slice is already borrowed")]
    HomeSliceBorrowed { operation: &'static str },

    /// A namespace-local identifier slice cannot represent another allocation.
    #[error("the {0:?} identifier namespace exhausted its u16 allocation range")]
    NamespaceCapacity(IdentifierNamespace),

    /// An archive's validated name-cardinality metadata exceeds the namespace's
    /// representable identifier range. This is checked before rkyv allocates a
    /// `Vec<Name>` during deserialization.
    #[error("the archived name slice declares {names} names, exceeding its u16 capacity")]
    ArchivedNamespaceCapacity { names: usize },

    /// The archive does not carry this boundary's magic envelope. Legacy raw
    /// rkyv payloads are deliberately unsupported.
    #[error("the name-table archive envelope is missing or corrupt")]
    InvalidArchiveEnvelope,

    /// The archive envelope names a wire layout this version does not support.
    #[error("the name-table archive version {found} is unsupported")]
    UnsupportedArchiveVersion { found: u16 },

    /// An archive contains two canonical names in one namespace slice.
    #[error("the archived name slice repeats canonical name {0:?}")]
    DuplicateCanonicalName(Name),

    /// Composition would make one canonical name point to two identifiers.
    #[error("canonical name {name:?} indexes both {first} and {second}")]
    NameIndexCollision {
        name: Name,
        first: Identifier,
        second: Identifier,
    },

    /// A slice descriptor's declared namespace disagrees with its archive.
    #[error("the snapshot declared {expected:?} but archives {actual:?}")]
    SnapshotNamespaceMismatch {
        expected: IdentifierNamespace,
        actual: IdentifierNamespace,
    },

    /// A slice descriptor's existing pin does not verify its unchanged archive.
    #[error("the {namespace:?} snapshot does not match its pinned slice identity")]
    SnapshotIdentityMismatch { namespace: IdentifierNamespace },

    /// Serializing the table's canonical name bytes failed.
    #[error("name-table serialization failed: {0}")]
    Serialize(String),

    /// Deserializing (with validation-on-read) the table's name bytes failed.
    #[error("name-table deserialization failed: {0}")]
    Deserialize(String),
}
