//! Typed failures of the generic nested-table boundary.

use thiserror::Error;

use crate::{
    EncodedId, LocalEncodedId, ModulePath, Name, NameReference, OperationKey, SnapshotDigest,
    StateRevision, TableAddress, TableMutability,
};

/// Constructing an encoded identity with the empty table-address chain.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("an encoded ID chain must contain at least one table-local ID")]
pub struct EmptyEncodedId;

/// A failure that leaves committed nested-table state unchanged.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum NameTableError<Root> {
    /// The same root table was submitted twice in one seal.
    #[error("one seal submitted the same root table more than once")]
    DuplicateRootDeclaration { root: Root },

    /// One submitted module declared the same exact spelling twice.
    #[error("one submitted module table contains a duplicate exact declaration spelling")]
    Redefinition {
        table: ModulePath<Root>,
        spelling: Name,
    },

    /// Existing table provisioning disagrees with the submitted mutability.
    #[error("a submitted table's mutability disagrees with its persisted head")]
    TableMutabilityMismatch {
        address: TableAddress<Root>,
        existing: TableMutability,
        submitted: TableMutability,
    },

    /// A table address does not exist.
    #[error("the requested table address does not exist")]
    UnknownTable { address: TableAddress<Root> },

    /// A lookup-only reference did not resolve.
    #[error("a lookup-only reference did not resolve")]
    UnresolvedReference { reference: NameReference<Root> },

    /// An encoded ID does not select an allocated entry.
    #[error("the encoded ID does not select an allocated table entry")]
    UnknownEncodedId { encoded_id: EncodedId<Root> },

    /// Rename was attempted against an immutable table.
    #[error("the owning table is immutable")]
    ImmutableTable { address: TableAddress<Root> },

    /// A rename would create a same-table spelling collision.
    #[error("the replacement spelling is already allocated in the owning table")]
    SpellingCollision {
        address: TableAddress<Root>,
        spelling: Name,
        existing: LocalEncodedId,
    },

    /// One table has allocated all 65,536 local IDs.
    #[error("the module table exhausted its u16 allocation range")]
    TableCapacity { address: TableAddress<Root> },

    /// A table generation cannot advance.
    #[error("the module table exhausted its generation counter")]
    TableGenerationExhausted { address: TableAddress<Root> },

    /// The pure state revision cannot advance.
    #[error("the nested-table state exhausted its revision counter")]
    StateRevisionExhausted,

    /// An idempotency key was reused with different operation content or kind.
    #[error("an idempotency key was reused with different operation content")]
    IdempotencyConflict { operation_key: OperationKey },

    /// A staged operation no longer matches the committed base revision.
    #[error("the staged operation was based on a stale state revision")]
    StaleStage {
        expected: StateRevision,
        actual: StateRevision,
    },

    /// Stored archive state contains an empty encoded-ID chain.
    #[error("stored state contains an empty encoded-ID chain")]
    EmptyStoredEncodedId,

    /// A stored child table has no allocated parent entry.
    #[error("a child table address has no allocated parent entry")]
    InconsistentTableAncestry { address: TableAddress<Root> },

    /// A stored head's key or current snapshot is inconsistent.
    #[error("a table head is inconsistent with its stored key or snapshot")]
    InconsistentTableHead { address: TableAddress<Root> },

    /// Stored snapshot entries are not a unique exact spelling map.
    #[error("a stored snapshot repeats an exact spelling")]
    DuplicateSnapshotSpelling {
        address: TableAddress<Root>,
        spelling: Name,
    },

    /// Stored snapshot cardinality exceeds the table-local range.
    #[error("a stored snapshot exceeds the table-local u16 range")]
    SnapshotCapacity { address: TableAddress<Root> },

    /// Stored snapshot integrity metadata does not verify.
    #[error("a stored snapshot does not match its integrity digest")]
    SnapshotIntegrityMismatch { digest: SnapshotDigest },

    /// Two archived records claim the same key.
    #[error("the archive contains duplicate keyed records")]
    DuplicateArchiveRecord,

    /// A durable receipt disagrees with its key or stored state.
    #[error("a durable operation receipt is inconsistent with stored state")]
    InconsistentReceipt { operation_key: OperationKey },

    /// The archive does not carry this crate's envelope.
    #[error("the nested name-table archive envelope is missing or corrupt")]
    InvalidArchiveEnvelope,

    /// The archive payload does not match its envelope integrity digest.
    #[error("the nested name-table archive payload failed integrity validation")]
    ArchiveIntegrityMismatch,

    /// The envelope selects a layout this crate does not support.
    #[error("the nested name-table archive version {found} is unsupported")]
    UnsupportedArchiveVersion { found: u16 },

    /// Canonical serialization failed.
    #[error("nested name-table serialization failed: {0}")]
    Serialize(String),

    /// Validated archive reconstruction failed.
    #[error("nested name-table deserialization failed: {0}")]
    Deserialize(String),
}
