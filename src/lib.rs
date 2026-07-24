//! The stringless-Core identifier space and its name interning.
//!
//! This is crate L2 of the shared-codec language family: `content-identity <-
//! name-table <- raw-discovery <- structural-codec`. Every `Core*` type in the
//! family is stringless — it carries [`Identifier`] values, never names — and
//! all names live here, in a [`NameTable`]. That is the substrate on which the
//! family's identity ruling stands: because a `Core` value holds no names, a
//! rename is a table-only edit that can never move `Core` content identity.
//!
//! ## What this crate owns
//!
//! - [`Identifier`] — a closed namespace enum whose variants carry `u16` local
//!   allocations, so identity is never flat-integer arithmetic.
//! - [`NameTable`] — one component's composed view: a component-owned home
//!   namespace plus complete borrowed read-only namespace slices, with
//!   [`intern`](NameTable::intern) and [`resolve`](NameTable::resolve). Composition
//!   preserves slices without copying, flattening, or renumbering identifiers.
//! - [`NameTransaction`] — a speculative interning overlay that merges on commit,
//!   so a failed decode alternative leaves no allocation effect (the accepted
//!   transactional-interning hardening).
//! - [`Name`] — the interned name and the ONE home of the derived-name rule
//!   ([`field_name`](Name::field_name), [`screaming`](Name::screaming),
//!   [`pascal_case`](Name::pascal_case)), consolidating walkers hand-written
//!   independently in `schema` and `schema-rust`.
//! - [`NameResolver`] / [`NameInterner`] — the two codec-boundary capabilities:
//!   the read-only view an encode path is threaded, and the mutating view a decode
//!   path is threaded.
//! - [`TextualProjection`] — the surface for deriving a named `Textual*` view from
//!   `Core` + a table. Concrete `Textual*` types belong to later crates.
//!
//! ## Names never serialize with Core values
//!
//! A table archives only its owned namespace slice and ordered canonical names;
//! borrowed slices remain independently archived. The lookup accelerator is
//! derived and never serialized. Content identity for a `Core` value comes from
//! `content-identity` over that value's stringless bytes,
//! which contain no names. So names and `Core` values are structurally incapable
//! of sharing a pre-image.

mod boundary;
mod error;
mod identifier;
mod name;
mod projection;
mod table;
mod transaction;

pub use boundary::{NameInterner, NameResolver};
pub use error::NameTableError;
pub use identifier::{Identifier, IdentifierNamespace};
pub use name::Name;
pub use projection::TextualProjection;
pub use table::{NameTable, NameTableDomain, NameTableSliceSnapshot};
pub use transaction::NameTransaction;
