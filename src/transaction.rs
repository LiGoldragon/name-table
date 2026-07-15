//! Transactional interning: a speculative overlay that merges on commit.
//!
//! The accepted hardening requires that a failed decode alternative leave NO
//! allocation effect on the table. This is met structurally rather than by
//! careful undo: a [`NameTransaction`] stages every new name on the side and
//! never mutates the committed table until [`commit`](NameTransaction::commit).
//! A dropped or [`rollback`](NameTransaction::rollback)ed transaction therefore
//! leaves the table byte-identical by construction — there is nothing to undo.

use std::collections::HashMap;

use crate::boundary::{NameInterner, NameResolver};
use crate::error::NameTableError;
use crate::identifier::Identifier;
use crate::name::Name;
use crate::table::NameTable;

/// A speculative interning overlay on a [`NameTable`]. Reads fall through to the
/// committed table; new names stage above it at the identifiers they would occupy
/// after a commit. The committed table is untouched until [`commit`] consumes the
/// transaction; dropping it instead is an implicit, effect-free rollback.
///
/// [`commit`]: NameTransaction::commit
pub struct NameTransaction<'table> {
    base: &'table mut NameTable,
    staged_names: Vec<Name>,
    staged_index: HashMap<Name, Identifier>,
}

impl<'table> NameTransaction<'table> {
    /// Open an overlay on `base`. The committed table is borrowed exclusively for
    /// the transaction's life but is not mutated until commit.
    pub(crate) fn new(base: &'table mut NameTable) -> Self {
        Self {
            base,
            staged_names: Vec::new(),
            staged_index: HashMap::new(),
        }
    }

    /// The first identifier a staged name would occupy — the committed table's
    /// current length. Stable for the transaction's life, since the committed
    /// table is borrowed exclusively and never mutated before commit.
    fn base_length(&self) -> usize {
        self.base.len()
    }

    /// Intern a name in the overlay. A name already committed resolves to its
    /// committed identifier (no staging); a name already staged resolves to its
    /// staged identifier; a genuinely new name stages at the next index above the
    /// committed table.
    pub fn intern(&mut self, name: Name) -> Identifier {
        if let Some(identifier) = self.base.lookup(&name) {
            return identifier;
        }
        if let Some(&identifier) = self.staged_index.get(&name) {
            return identifier;
        }
        let identifier = Identifier::new((self.base_length() + self.staged_names.len()) as u32);
        self.staged_names.push(name.clone());
        self.staged_index.insert(name, identifier);
        identifier
    }

    /// Resolve an identifier against the overlay: committed identifiers resolve
    /// through the committed table, staged identifiers through the staging buffer.
    pub fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        let base_length = self.base_length();
        if identifier.position() < base_length {
            return self.base.resolve(identifier);
        }
        self.staged_names
            .get(identifier.position() - base_length)
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// The number of names staged but not yet committed.
    pub fn staged_count(&self) -> usize {
        self.staged_names.len()
    }

    /// Merge the staged names into the committed table, keeping their staged
    /// identifiers. This is the only path that mutates the committed table.
    pub fn commit(self) {
        let Self {
            base, staged_names, ..
        } = self;
        base.commit_staged(staged_names);
    }

    /// Discard the staged names, leaving the committed table untouched. Identical
    /// in effect to dropping the transaction; named for call sites that want the
    /// rollback to read explicitly.
    pub fn rollback(self) {}
}

impl NameResolver for NameTransaction<'_> {
    fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        NameTransaction::resolve(self, identifier)
    }
}

impl NameInterner for NameTransaction<'_> {
    fn intern(&mut self, name: Name) -> Identifier {
        NameTransaction::intern(self, name)
    }
}
