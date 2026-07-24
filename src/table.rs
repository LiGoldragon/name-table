//! The composable interning identifier space.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use content_identity::{ContentHash, DomainSeparation, HashDomain, LayoutVersion, PortableArchive};

use crate::boundary::{NameInterner, NameResolver};
use crate::error::NameTableError;
use crate::identifier::{Identifier, IdentifierNamespace};
use crate::name::Name;
use crate::transaction::NameTransaction;

/// The hash domain of one namespace slice's content identity.
///
/// A slice has its own identity because a component borrows other slices instead
/// of copying them. The composed view's borrowed edges are runtime topology;
/// they are not duplicated into the home slice's stored content.
pub struct NameTableDomain;

impl HashDomain for NameTableDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "name-table 2026 sliced identifier space",
            layout: LayoutVersion::new(3),
        }
    }
}

/// One namespace's owned canonical names.
///
/// Each encoded identifier has exactly one canonical name in its component
/// projection. Additional source or target-language spellings are not part of
/// the language surface.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct NameSlice {
    namespace: IdentifierNamespace,
    names: Vec<Name>,
}

/// The storage-wire version of one home-slice archive.
///
/// This is intentionally separate from [`NameTableDomain`]'s hash layout
/// version: the former selects a persisted byte decoder, while the latter
/// separates content identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NameTableArchiveVersion(u16);

impl NameTableArchiveVersion {
    const CURRENT: Self = Self(1);
}

/// The typed storage-wire header for one home-slice archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NameTableArchiveEnvelope {
    version: NameTableArchiveVersion,
}

impl NameTableArchiveEnvelope {
    const MAGIC: [u8; 8] = *b"NTABLE\0\0";
    const HEADER_LEN: usize = Self::MAGIC.len() + std::mem::size_of::<u16>();

    const fn current() -> Self {
        Self {
            version: NameTableArchiveVersion::CURRENT,
        }
    }

    fn seal(self, payload: &[u8]) -> rkyv::util::AlignedVec {
        let mut bytes = rkyv::util::AlignedVec::with_capacity(Self::HEADER_LEN + payload.len());
        bytes.extend_from_slice(&Self::MAGIC);
        bytes.extend_from_slice(&self.version.0.to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn open(bytes: &[u8]) -> Result<&[u8], NameTableError> {
        if bytes.len() < Self::HEADER_LEN || bytes[..Self::MAGIC.len()] != Self::MAGIC {
            return Err(NameTableError::InvalidArchiveEnvelope);
        }

        let version_bytes: [u8; std::mem::size_of::<u16>()] = bytes
            [Self::MAGIC.len()..Self::HEADER_LEN]
            .try_into()
            .map_err(|_| NameTableError::InvalidArchiveEnvelope)?;
        let version = NameTableArchiveVersion(u16::from_le_bytes(version_bytes));
        if version != NameTableArchiveVersion::CURRENT {
            return Err(NameTableError::UnsupportedArchiveVersion { found: version.0 });
        }

        Ok(&bytes[Self::HEADER_LEN..])
    }
}

impl NameSlice {
    fn new(namespace: IdentifierNamespace) -> Self {
        Self {
            namespace,
            names: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.names.len()
    }

    fn name(&self, local: u16) -> Option<&Name> {
        self.names.get(usize::from(local))
    }

    fn identifier_at(&self, position: usize) -> Result<Identifier, NameTableError> {
        let local = u16::try_from(position)
            .map_err(|_| NameTableError::NamespaceCapacity(self.namespace))?;
        Ok(self.namespace.identifier(local))
    }

    fn canonical_identifiers(&self) -> Result<Vec<(Name, Identifier)>, NameTableError> {
        self.validate()?;
        self.names
            .iter()
            .enumerate()
            .map(|(position, name)| Ok((name.clone(), self.identifier_at(position)?)))
            .collect()
    }

    fn validate(&self) -> Result<(), NameTableError> {
        if self.names.len() > usize::from(u16::MAX) + 1 {
            return Err(NameTableError::NamespaceCapacity(self.namespace));
        }

        let mut canonical_names = HashSet::with_capacity(self.names.len());
        for name in &self.names {
            if !canonical_names.insert(name) {
                return Err(NameTableError::DuplicateCanonicalName(name.clone()));
            }
        }
        Ok(())
    }

    fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        let identifier = self.identifier_at(self.names.len())?;
        self.names.push(name);
        Ok(identifier)
    }
}

/// One independently archived, namespace-tagged slice in a composed nametree.
///
/// The bytes are exactly the existing versioned home-slice archive for this
/// namespace. A Capsule carries these descriptors separately instead of
/// flattening borrowed slices into its home archive.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct NameTableSliceSnapshot {
    namespace: IdentifierNamespace,
    archive_bytes: Vec<u8>,
    identity: ContentHash<NameTableDomain>,
}

impl NameTableSliceSnapshot {
    fn from_slice(slice: &NameSlice) -> Result<Self, NameTableError> {
        let payload = slice
            .to_archive_bytes()
            .map_err(|error| NameTableError::Serialize(error.to_string()))?;
        let archive_bytes = NameTableArchiveEnvelope::current()
            .seal(payload.as_ref())
            .to_vec();
        let identity = ContentHash::<NameTableDomain>::of_core(slice)
            .map_err(|error| NameTableError::Serialize(error.to_string()))?;
        Ok(Self {
            namespace: slice.namespace,
            archive_bytes,
            identity,
        })
    }

    /// The namespace this independently pinned slice owns.
    pub fn namespace(&self) -> IdentifierNamespace {
        self.namespace
    }

    /// The slice's unchanged, versioned home-archive bytes.
    pub fn archive_bytes(&self) -> &[u8] {
        &self.archive_bytes
    }

    /// The existing identity of this one namespace slice.
    pub fn identity(&self) -> ContentHash<NameTableDomain> {
        self.identity
    }

    fn restore(&self) -> Result<NameTable, NameTableError> {
        let table = NameTable::from_archive_bytes(&self.archive_bytes)?;
        if table.namespace() != self.namespace {
            return Err(NameTableError::SnapshotNamespaceMismatch {
                expected: self.namespace,
                actual: table.namespace(),
            });
        }
        let actual = table.identity()?;
        if actual != self.identity {
            return Err(NameTableError::SnapshotIdentityMismatch {
                namespace: self.namespace,
            });
        }
        Ok(table)
    }
}

/// An interned, composable map from [`Identifier`] to [`Name`].
///
/// A component owns exactly one home namespace and may borrow complete,
/// read-only slices from other components. Composition stores shared `Arc`
/// handles to those source slices; it does not copy source names, flatten source
/// state, or renumber source identifiers. `intern` always allocates in the home
/// namespace, while `resolve` dispatches exhaustively by identifier variant.
///
/// A source component completes its own allocations before another component
/// borrows its slice. That lifecycle keeps the borrowed view immutable and makes
/// namespace-local identifiers stable.
///
/// `NameTable` implements [`Clone`] because the structural codec's public `Converted`
/// output is cloneable. A clone copies only the derived lookup accelerator; its home and
/// every borrowed source slice remain the same `Arc` handles. Sharing the home `Arc`
/// deliberately seals it in both values instead of using copy-on-write: clones cannot
/// acquire divergent mutable ownership of one component namespace. Cloning never
/// flattens, renumbers, or copies a borrowed namespace slice, and preserves the
/// sealed-home lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameTable {
    home: Arc<NameSlice>,
    borrowed: BTreeMap<IdentifierNamespace, Arc<NameSlice>>,
    index: HashMap<Name, Identifier>,
}

impl NameTable {
    /// Create one component's empty NameTable with its owned allocation slice.
    pub fn new(namespace: IdentifierNamespace) -> Self {
        Self {
            home: Arc::new(NameSlice::new(namespace)),
            borrowed: BTreeMap::new(),
            index: HashMap::new(),
        }
    }

    fn from_slices(
        home: Arc<NameSlice>,
        borrowed: BTreeMap<IdentifierNamespace, Arc<NameSlice>>,
    ) -> Result<Self, NameTableError> {
        let mut table = Self {
            home,
            borrowed,
            index: HashMap::new(),
        };
        table.rebuild_index()?;
        Ok(table)
    }

    /// The namespace this component owns and appends to.
    pub fn namespace(&self) -> IdentifierNamespace {
        self.home.namespace
    }

    /// The number of names owned by this component's home slice.
    pub fn len(&self) -> usize {
        self.home.len()
    }

    /// Whether this component owns no names yet.
    pub fn is_empty(&self) -> bool {
        self.home.names.is_empty()
    }

    /// Borrow every completed source slice into this component's composed table.
    ///
    /// The source's names retain their original identifiers. A composed source
    /// contributes both its home and every slice it already borrows, each by its
    /// existing `Arc` handle. Borrowing a namespace already represented here is
    /// rejected, because an identifier variant has one authoritative slice.
    pub fn compose(&self, source: &NameTable) -> Result<Self, NameTableError> {
        let mut borrowed = self.borrowed.clone();
        let source_slices =
            std::iter::once((&source.home.namespace, &source.home)).chain(source.borrowed.iter());

        for (namespace, slice) in source_slices {
            if *namespace == self.namespace() || borrowed.contains_key(namespace) {
                return Err(NameTableError::DuplicateNamespace(*namespace));
            }
            borrowed.insert(*namespace, Arc::clone(slice));
        }

        Self::from_slices(Arc::clone(&self.home), borrowed)
    }

    /// Intern a canonical name into this component's home slice.
    ///
    /// A canonical source name already present in any composed slice resolves to
    /// that existing identifier without reintroducing a string into Nomos.
    pub fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        if let Some(identifier) = self.index.get(&name).copied() {
            return Ok(identifier);
        }

        let home = Arc::get_mut(&mut self.home).ok_or(NameTableError::HomeSliceBorrowed {
            operation: "intern a name",
        })?;
        let identifier = home.intern(name.clone())?;
        self.index.insert(name, identifier);
        Ok(identifier)
    }

    /// Resolve an identifier to its canonical primary name.
    pub fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        self.slice(identifier.namespace())?
            .name(identifier.local())
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// Look up a canonical name without allocating.
    pub fn lookup(&self, name: &Name) -> Option<Identifier> {
        self.index.get(name).copied()
    }

    fn slice(&self, namespace: IdentifierNamespace) -> Result<&NameSlice, NameTableError> {
        if namespace == self.namespace() {
            return Ok(self.home.as_ref());
        }
        self.borrowed
            .get(&namespace)
            .map(Arc::as_ref)
            .ok_or(NameTableError::UnknownNamespace(namespace))
    }

    fn rebuild_index(&mut self) -> Result<(), NameTableError> {
        let mut identifiers = self.home.canonical_identifiers()?;
        for slice in self.borrowed.values() {
            identifiers.extend(slice.canonical_identifiers()?);
        }

        self.index.clear();
        for (name, identifier) in identifiers {
            self.insert_indexed_name(name, identifier)?;
        }
        Ok(())
    }

    fn insert_indexed_name(
        &mut self,
        name: Name,
        identifier: Identifier,
    ) -> Result<(), NameTableError> {
        if let Some(existing) = self.index.insert(name.clone(), identifier) {
            return Err(NameTableError::NameIndexCollision {
                name,
                first: existing,
                second: identifier,
            });
        }
        Ok(())
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
    /// effect: the table is byte-identical to before the call.
    pub fn try_intern<Value, Failure>(
        &mut self,
        attempt: impl FnOnce(&mut NameTransaction<'_>) -> Result<Value, Failure>,
    ) -> Result<Value, Failure>
    where
        Failure: From<NameTableError>,
    {
        let mut transaction = self.begin();
        match attempt(&mut transaction) {
            Ok(value) => {
                transaction.commit().map_err(Failure::from)?;
                Ok(value)
            }
            Err(failure) => {
                transaction.rollback();
                Err(failure)
            }
        }
    }

    /// Merge a transaction's staged names into the home slice.
    pub(crate) fn commit_staged(&mut self, staged: Vec<Name>) -> Result<(), NameTableError> {
        for name in staged {
            self.intern(name)?;
        }
        Ok(())
    }

    /// The home slice's versioned canonical archive bytes. Borrowed slices are
    /// deliberately excluded: they remain independently content-identified and
    /// are composed again by their consumer rather than copied into this
    /// component's state.
    pub fn to_archive_bytes(&self) -> Result<rkyv::util::AlignedVec, NameTableError> {
        let payload = self
            .home
            .as_ref()
            .to_archive_bytes()
            .map_err(|error| NameTableError::Serialize(error.to_string()))?;
        Ok(NameTableArchiveEnvelope::current().seal(payload.as_ref()))
    }

    /// Reconstruct one uncomposed home slice from versioned canonical archive
    /// bytes. Consumers compose required borrowed slices explicitly after
    /// loading. Legacy raw rkyv bytes are rejected rather than bridged.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, NameTableError> {
        let payload = NameTableArchiveEnvelope::open(bytes)?;
        let archived = rkyv::access::<ArchivedNameSlice, rkyv::rancor::Error>(payload)
            .map_err(|error| NameTableError::Deserialize(error.to_string()))?;
        let names = archived.names.len();
        if names > usize::from(u16::MAX) + 1 {
            return Err(NameTableError::ArchivedNamespaceCapacity { names });
        }

        let home = NameSlice::from_archive_bytes(payload)
            .map_err(|error| NameTableError::Deserialize(error.to_string()))?;
        home.validate()?;
        Self::from_slices(Arc::new(home), BTreeMap::new())
    }

    /// This home slice's content identity. Borrowed slices retain their own
    /// identities and are not folded into this component's owned name data.
    pub fn identity(&self) -> Result<ContentHash<NameTableDomain>, NameTableError> {
        ContentHash::<NameTableDomain>::of_core(self.home.as_ref())
            .map_err(|error| NameTableError::Serialize(error.to_string()))
    }

    /// Export the complete composed nametree as independently archived slices.
    ///
    /// The first value is the one owned home slice. The remaining values are
    /// borrowed source slices in deterministic namespace order. Each archive is
    /// the existing per-slice archive unchanged; this boundary never flattens,
    /// copies, renumbers, or restamps names.
    pub fn slice_snapshots(
        &self,
    ) -> Result<(NameTableSliceSnapshot, Vec<NameTableSliceSnapshot>), NameTableError> {
        let home = NameTableSliceSnapshot::from_slice(self.home.as_ref())?;
        let borrowed = self
            .borrowed
            .values()
            .map(|slice| NameTableSliceSnapshot::from_slice(slice.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((home, borrowed))
    }

    /// Restore a complete composed nametree from independently pinned slice
    /// snapshots. The home remains owned and every other slice is borrowed by
    /// composition; no snapshot is flattened into or reallocated by another.
    pub fn from_slice_snapshots(
        home: &NameTableSliceSnapshot,
        borrowed: &[NameTableSliceSnapshot],
    ) -> Result<Self, NameTableError> {
        let mut restored = home.restore()?;
        let mut namespaces = HashSet::new();
        namespaces.insert(restored.namespace());
        for snapshot in borrowed {
            if !namespaces.insert(snapshot.namespace()) {
                return Err(NameTableError::DuplicateNamespace(snapshot.namespace()));
            }
            restored = restored.compose(&snapshot.restore()?)?;
        }
        Ok(restored)
    }
}

impl NameResolver for NameTable {
    fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        NameTable::resolve(self, identifier)
    }
}

impl NameInterner for NameTable {
    fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        NameTable::intern(self, name)
    }
}

#[cfg(test)]
mod archive_tests {
    use content_identity::PortableArchive;

    use super::{Name, NameSlice, NameTable, NameTableArchiveEnvelope};
    use crate::error::NameTableError;
    use crate::identifier::IdentifierNamespace;

    #[test]
    fn oversized_validated_metadata_is_rejected_before_name_deserialization() {
        // This is deliberately only one namespace beyond its u16 range, not a
        // million-name construction. `from_archive_bytes` reports the archived
        // cardinality before rkyv can allocate a deserialized `Vec<Name>`.
        let names = usize::from(u16::MAX) + 2;
        let oversized = NameSlice {
            namespace: IdentifierNamespace::Schema,
            names: vec![Name::new("Repeated"); names],
        };
        let payload = oversized
            .to_archive_bytes()
            .expect("archive oversized slice");
        let bytes = NameTableArchiveEnvelope::current().seal(payload.as_ref());

        assert!(matches!(
            NameTable::from_archive_bytes(bytes.as_ref()),
            Err(NameTableError::ArchivedNamespaceCapacity { names: archived_names })
                if archived_names == names
        ));
    }
}
