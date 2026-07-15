//! The interning identifier space.

use std::collections::HashMap;

use content_identity::{ContentHash, DomainSeparation, HashDomain, LayoutVersion, PortableArchive};

use crate::boundary::{NameInterner, NameResolver};
use crate::error::NameTableError;
use crate::identifier::Identifier;
use crate::name::Name;
use crate::transaction::NameTransaction;

/// The hash domain of a name table's own content identity.
///
/// A table has its own identity so it can be stored as a co-versioned sibling of
/// the `Core` values whose identifiers it resolves. This is the table's identity,
/// entirely separate from any `Core` value's identity: a `Core` hash never folds
/// a name (names are not in a `Core` value), so this domain and the Core domains
/// never meet.
pub struct NameTableDomain;

impl HashDomain for NameTableDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "name-table 2026 interned identifier space",
            layout: LayoutVersion::new(1),
        }
    }
}

/// An interned, append-only map from [`Identifier`] to [`Name`].
///
/// Interning is deterministic and index-stable: a name interns to the same
/// identifier every time within one table lineage, and an identifier's index
/// never changes once allocated. That stability is what makes [`extend_from`] a
/// continuous identifier space — a logos table that extends a schema table keeps
/// every schema identifier at its exact index.
///
/// The canonical, archivable state is the ordered name vector alone; the lookup
/// index is a derived accelerator, rebuilt on load and never serialized. So a
/// table's bytes are its names and nothing else — names and `Core` values can
/// never serialize together.
///
/// [`extend_from`]: NameTable::extend_from
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NameTable {
    /// The interned names in identifier order; index `i` is `Identifier::new(i)`.
    names: Vec<Name>,
    /// A derived name-to-identifier accelerator; rebuilt on load, never archived.
    index: HashMap<Name, Identifier>,
}

impl NameTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of interned names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the table has interned nothing.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Intern a name, returning its identifier and allocating a new index only if
    /// the name is unseen. Deterministic: the same name always returns the same
    /// identifier.
    pub fn intern(&mut self, name: Name) -> Identifier {
        if let Some(&existing) = self.index.get(&name) {
            return existing;
        }
        let identifier = Identifier::new(self.names.len() as u32);
        self.names.push(name.clone());
        self.index.insert(name, identifier);
        identifier
    }

    /// Resolve an identifier back to its name.
    pub fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        self.names
            .get(identifier.position())
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// The identifier a name already holds, without interning it. Used by a
    /// speculative transaction to fall through to the committed table.
    pub(crate) fn lookup(&self, name: &Name) -> Option<Identifier> {
        self.index.get(name).copied()
    }

    /// Build a new table that extends `base`: it begins with every name of `base`
    /// at its exact identifier, so a carried-over identifier resolves identically
    /// in the extension, and new names append at higher indices. This is the one
    /// continuous identifier space that carries schema's allocation into logos.
    pub fn extend_from(base: &NameTable) -> NameTable {
        base.clone()
    }

    /// Open a speculative transaction over this table. Names interned through the
    /// returned [`NameTransaction`] stage on the side; the committed table is not
    /// touched until [`NameTransaction::commit`]. Dropping the transaction (or
    /// calling [`NameTransaction::rollback`]) leaves this table byte-identical.
    pub fn begin(&mut self) -> NameTransaction<'_> {
        NameTransaction::new(self)
    }

    /// Run `attempt` against a speculative transaction, committing its interned
    /// names only if it succeeds. A failed alternative leaves no allocation
    /// effect: the table is byte-identical to before the call. This is the
    /// transactional-interning contract a decode alternative uses so a failed
    /// decode never leaks a name.
    pub fn try_intern<Value, Failure>(
        &mut self,
        attempt: impl FnOnce(&mut NameTransaction<'_>) -> Result<Value, Failure>,
    ) -> Result<Value, Failure> {
        let mut transaction = self.begin();
        match attempt(&mut transaction) {
            Ok(value) => {
                transaction.commit();
                Ok(value)
            }
            Err(failure) => {
                transaction.rollback();
                Err(failure)
            }
        }
    }

    /// Merge a transaction's staged names into the committed table, preserving
    /// their staged identifiers (which were allocated above `self.len()`).
    pub(crate) fn commit_staged(&mut self, staged: Vec<Name>) {
        for name in staged {
            self.intern(name);
        }
    }

    /// The table's canonical rkyv bytes: the ordered names, and only the names.
    /// The derived lookup index is not part of the pre-image.
    pub fn to_archive_bytes(&self) -> Result<rkyv::util::AlignedVec, NameTableError> {
        self.names
            .to_archive_bytes()
            .map_err(|error| NameTableError::Serialize(error.to_string()))
    }

    /// Reconstruct a table from its canonical name bytes, rebuilding the derived
    /// lookup index deterministically.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, NameTableError> {
        let names = Vec::<Name>::from_archive_bytes(bytes)
            .map_err(|error| NameTableError::Deserialize(error.to_string()))?;
        let mut table = NameTable::new();
        for name in names {
            table.intern(name);
        }
        Ok(table)
    }

    /// The table's own content identity, over its canonical name bytes. A table is
    /// storable as a co-versioned sibling of the `Core` values it names; this is
    /// the address it would be stored under. Excluded by construction from any
    /// `Core` value's identity, because a `Core` value holds no names.
    pub fn identity(&self) -> Result<ContentHash<NameTableDomain>, NameTableError> {
        ContentHash::<NameTableDomain>::of_core(&self.names)
            .map_err(|error| NameTableError::Serialize(error.to_string()))
    }
}

impl NameResolver for NameTable {
    fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        NameTable::resolve(self, identifier)
    }
}

impl NameInterner for NameTable {
    fn intern(&mut self, name: Name) -> Identifier {
        NameTable::intern(self, name)
    }
}
