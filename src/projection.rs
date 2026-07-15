//! The Textual-projection surface.
//!
//! A named (`Textual*`) view is DERIVED from a stringless `Core` value plus a
//! [`NameTable`], never stored. Because the projection is computed on demand and
//! the name text lives only in the table, a rename is a table-only edit that
//! never touches `Core` identity — the projection just resolves differently next
//! time it runs.
//!
//! This crate provides the surface only; concrete `Textual*` types belong to
//! later crates, which implement [`TextualProjection`] for their `Core`/`Textual`
//! pair.
//!
//! [`NameTable`]: crate::NameTable

use crate::boundary::NameResolver;
use crate::error::NameTableError;

/// Derives a named view of a stringless `Core` value by resolving its identifiers
/// through a [`NameResolver`]. The derived view is never stored on the `Core`
/// value, so the two never serialize together.
pub trait TextualProjection {
    /// The stringless `Core` value this projects from.
    type Core;
    /// The derived named view. A concrete `Textual*` type in a later crate.
    type Textual;

    /// Derive the named view of `core`, resolving every identifier through
    /// `names`. Fails with [`NameTableError::UnknownIdentifier`] if `core` carries
    /// an identifier the resolver does not know — a torn `Core`/`NameTable` pair.
    fn project<Resolver>(
        core: &Self::Core,
        names: &Resolver,
    ) -> Result<Self::Textual, NameTableError>
    where
        Resolver: NameResolver;
}
