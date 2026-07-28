//! Generic nested module-owned name tables.
//!
//! A root value selects one root table without this crate defining the
//! production root set. Each module is an entry in its containing table and,
//! structurally, owns the child table at that entry's complete [`EncodedId`].
//! Every durable identity is therefore a root plus a non-empty chain of
//! table-local [`LocalEncodedId`] values.
//!
//! Declarations allocate exact, case-sensitive spellings within one table.
//! References only resolve. [`SealRequest`] applies a typed nested declaration
//! graph through pure staging, and [`RenameRequest`] changes one spelling while
//! preserving the target chain and all descendant chains.
//!
//! Archive version 3 adds whole-payload integrity and exact changed-snapshot
//! receipt records. Earlier archives are rejected rather than guessed into
//! module chains or replay results.

mod archive;
mod error;
mod identity;
mod name;
mod request;
mod state;
mod table;
mod transaction;

pub use error::{EmptyEncodedId, NameTableError};
pub use identity::{EncodedId, LocalEncodedId, TableAddress};
pub use name::Name;
pub use request::{
    Declaration, ModuleDeclaration, ModulePath, NamePath, NameReference, RenameRequest,
    RootDeclaration, SealRequest,
};
pub use state::{
    AllocationCursor, ModuleTableHead, ModuleTableSnapshot, OperationKey, RenameReceipt,
    RequestDigest, ResolvedName, SealReceipt, SnapshotDigest, StateRevision, TableChange,
    TableGeneration, TableMutability,
};
pub use table::NameTable;
pub use transaction::{StagedRename, StagedSeal};
