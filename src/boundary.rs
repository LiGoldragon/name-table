//! The two codec-boundary capabilities.
//!
//! A codec never holds the whole [`NameTable`]; it is threaded exactly the
//! capability its direction needs. Encode reads names, so it takes a
//! [`NameResolver`]; decode allocates names, so it takes a [`NameInterner`]. The
//! table implements both; a speculative [`NameTransaction`] implements
//! [`NameInterner`] so a decode alternative can allocate without touching the
//! committed table (see `crate::transaction`).
//!
//! [`NameTable`]: crate::NameTable
//! [`NameTransaction`]: crate::NameTransaction

use crate::error::NameTableError;
use crate::identifier::Identifier;
use crate::name::Name;

/// The read-only view an encode path is given: resolve an [`Identifier`] back to
/// its [`Name`]. Threaded down the encode call tree, never held by a node.
pub trait NameResolver {
    /// The name interned for `identifier`, or [`NameTableError::UnknownIdentifier`]
    /// if the identifier does not belong to this table.
    fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError>;
}

/// The mutating view a decode path is given: intern a [`Name`] to an
/// [`Identifier`], allocating into the continuous identifier space when the name
/// is new. Threaded down the decode call tree, never held by a node.
pub trait NameInterner {
    /// The identifier for `name`, interning it if it has not been seen. Interning
    /// is deterministic: the same name always yields the same identifier within
    /// one table lineage.
    fn intern(&mut self, name: Name) -> Identifier;
}
