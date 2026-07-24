//! The Textual-projection surface.
//!
//! A named (`Textual*`) view is DERIVED from a stringless encoded value plus a
//! [`NameTable`], never stored. Because the projection is computed on demand and
//! the name text lives only in the table, a rename is a table-only edit that
//! never touches encoded identity — the projection just resolves differently next
//! time it runs.
//!
//! This crate provides the surface only; concrete `Textual*` types belong to
//! later crates, which implement [`TextualProjection`] for their encoded/Textual
//! pair.
//!
//! [`NameTable`]: crate::NameTable

use crate::boundary::NameResolver;
use crate::error::NameTableError;

/// Derives a named view of a stringless encoded value by resolving its identifiers
/// through a [`NameResolver`]. The derived view is never stored on the encoded
/// value, so the two never serialize together.
pub trait TextualProjection {
    /// The stringless encoded value this projects from.
    type Encoded;
    /// The derived named view. A concrete `Textual*` type in a later crate.
    type Textual;

    /// Derive the named view of `encoded`, resolving every identifier through
    /// `names`. Fails with [`NameTableError::UnknownIdentifier`] if `encoded` carries
    /// an identifier the resolver does not know — a torn encoded/`NameTable` pair.
    fn project<Resolver>(
        encoded: &Self::Encoded,
        names: &Resolver,
    ) -> Result<Self::Textual, NameTableError>
    where
        Resolver: NameResolver;
}
