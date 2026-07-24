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
/// complete composed table; new names stage only in its owned home namespace.
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

    /// The first local a staged name would occupy in the home namespace.
    fn base_length(&self) -> usize {
        self.base.len()
    }

    /// Intern a name in the overlay. A name already present in the home or any
    /// borrowed slice resolves to its existing identifier; a new spelling stages
    /// in the home namespace without allocating into a borrowed source.
    pub fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        if let Some(identifier) = self.base.lookup(&name) {
            return Ok(identifier);
        }
        if let Some(&identifier) = self.staged_index.get(&name) {
            return Ok(identifier);
        }
        let local = u16::try_from(self.base_length() + self.staged_names.len())
            .map_err(|_| NameTableError::NamespaceCapacity(self.base.namespace()))?;
        let identifier = self.base.namespace().identifier(local);
        self.staged_names.push(name.clone());
        self.staged_index.insert(name, identifier);
        Ok(identifier)
    }

    /// Resolve an identifier against the overlay: borrowed and committed
    /// identifiers resolve through the composed base; staged home identifiers
    /// resolve through the staging buffer.
    pub fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        if identifier.namespace() != self.base.namespace()
            || usize::from(identifier.local()) < self.base_length()
        {
            return self.base.resolve(identifier);
        }
        self.staged_names
            .get(usize::from(identifier.local()) - self.base_length())
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// The number of names staged but not yet committed.
    pub fn staged_count(&self) -> usize {
        self.staged_names.len()
    }

    /// Merge the staged names into the committed table, keeping their staged
    /// namespace-local identifiers. This is the only path that mutates the home
    /// slice.
    pub fn commit(self) -> Result<(), NameTableError> {
        let Self {
            base, staged_names, ..
        } = self;
        base.commit_staged(staged_names)
    }

    /// Discard the staged names, leaving the composed table untouched.
    pub fn rollback(self) {}
}

impl NameResolver for NameTransaction<'_> {
    fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        NameTransaction::resolve(self, identifier)
    }
}

impl NameInterner for NameTransaction<'_> {
    fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        NameTransaction::intern(self, name)
    }
}
