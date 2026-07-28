//! Persistent state records and typed digests.

use content_identity::{DomainSeparation, HashDomain, IntegrityDigest, LayoutVersion};

use crate::{EncodedId, LocalEncodedId, Name, NamePath, TableAddress};

/// Whether the entries in one table may be operationally renamed.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum TableMutability {
    /// Operational rename is permitted.
    Mutable,
    /// Operational rename fails before target lookup.
    Immutable,
}

/// One immutable table generation.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct TableGeneration(u64);

impl TableGeneration {
    /// The initial empty snapshot generation.
    pub const ZERO: Self = Self(0);

    /// The integer generation.
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// The next table-local allocation, explicitly representing exhaustion.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum AllocationCursor {
    /// The next available local ID.
    Next(LocalEncodedId),
    /// Every `u16` local ID has been allocated.
    Exhausted,
}

impl AllocationCursor {
    /// The cursor of a new empty table.
    pub const ZERO: Self = Self::Next(LocalEncodedId::new(0));

    pub(crate) fn allocate(self) -> Option<(LocalEncodedId, Self)> {
        match self {
            Self::Next(local) if local.value() == u16::MAX => Some((local, Self::Exhausted)),
            Self::Next(local) => Some((local, Self::Next(LocalEncodedId::new(local.value() + 1)))),
            Self::Exhausted => None,
        }
    }

    pub(crate) fn for_entry_count(entries: usize) -> Option<Self> {
        match entries {
            0..=65535 => Some(Self::Next(LocalEncodedId::new(entries as u16))),
            65536 => Some(Self::Exhausted),
            _ => None,
        }
    }
}

/// The pure library state's monotonically increasing revision.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct StateRevision(u64);

impl StateRevision {
    /// The revision integer.
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// A caller-supplied operation idempotency key.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct OperationKey([u8; 32]);

impl OperationKey {
    /// Construct a key from caller-owned bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The exact key bytes.
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Typed domain for an immutable snapshot's integrity digest.
pub struct SnapshotIntegrityDomain;

impl HashDomain for SnapshotIntegrityDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "name-table module snapshot integrity",
            layout: LayoutVersion::new(1),
        }
    }
}

/// Typed domain for the canonical semantic content of a seal request.
pub struct SealRequestDomain;

impl HashDomain for SealRequestDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "name-table canonical nested seal request",
            layout: LayoutVersion::new(1),
        }
    }
}

/// Integrity locator for one immutable table snapshot.
///
/// This digest is caching and corruption-detection metadata only.
pub type SnapshotDigest = IntegrityDigest<SnapshotIntegrityDomain>;

/// Digest of a seal's canonical nested graph, excluding its idempotency key.
pub type RequestDigest = IntegrityDigest<SealRequestDomain>;

/// One table generation and immutable snapshot created by a successful seal.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TableChange<Root> {
    pub(crate) address: TableAddress<Root>,
    pub(crate) generation: TableGeneration,
    pub(crate) snapshot: SnapshotDigest,
}

impl<Root> TableChange<Root> {
    /// The table whose committed head changed.
    pub fn address(&self) -> &TableAddress<Root> {
        &self.address
    }

    /// The resulting immutable generation.
    pub fn generation(&self) -> TableGeneration {
        self.generation
    }

    /// The exact resulting immutable snapshot locator.
    pub fn snapshot(&self) -> SnapshotDigest {
        self.snapshot
    }
}

/// The current mutable head of one table.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ModuleTableHead<Root> {
    pub(crate) address: TableAddress<Root>,
    pub(crate) mutability: TableMutability,
    pub(crate) generation: TableGeneration,
    pub(crate) allocation_cursor: AllocationCursor,
    pub(crate) current_snapshot: SnapshotDigest,
}

impl<Root> ModuleTableHead<Root> {
    /// The table's structural address.
    pub fn address(&self) -> &TableAddress<Root> {
        &self.address
    }

    /// The table-local operational rename policy.
    pub fn mutability(&self) -> TableMutability {
        self.mutability
    }

    /// The current immutable generation.
    pub fn generation(&self) -> TableGeneration {
        self.generation
    }

    /// The explicit next-or-exhausted cursor.
    pub fn allocation_cursor(&self) -> AllocationCursor {
        self.allocation_cursor
    }

    /// The current immutable snapshot locator.
    pub fn current_snapshot(&self) -> SnapshotDigest {
        self.current_snapshot
    }
}

/// One immutable generation of one table's ordered exact spellings.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ModuleTableSnapshot<Root> {
    pub(crate) address: TableAddress<Root>,
    pub(crate) generation: TableGeneration,
    pub(crate) entries: Vec<Name>,
    pub(crate) digest: SnapshotDigest,
}

impl<Root> ModuleTableSnapshot<Root> {
    /// The table's structural address.
    pub fn address(&self) -> &TableAddress<Root> {
        &self.address
    }

    /// This immutable generation.
    pub fn generation(&self) -> TableGeneration {
        self.generation
    }

    /// Ordered `local encodedID -> exact spelling` entries.
    pub fn entries(&self) -> &[Name] {
        &self.entries
    }

    /// The snapshot's integrity locator.
    pub fn digest(&self) -> SnapshotDigest {
        self.digest
    }

    /// Resolve one table-local ID.
    pub fn resolve_local(&self, local: LocalEncodedId) -> Option<&Name> {
        self.entries.get(usize::from(local.value()))
    }

    /// Look up an exact spelling in this table only.
    pub fn lookup_local(&self, spelling: &Name) -> Option<LocalEncodedId> {
        self.entries
            .iter()
            .position(|candidate| candidate == spelling)
            .and_then(|position| u16::try_from(position).ok())
            .map(LocalEncodedId::new)
    }
}

/// One declared or referenced path resolved by a seal.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ResolvedName<Root> {
    pub(crate) path: NamePath<Root>,
    pub(crate) encoded_id: EncodedId<Root>,
}

impl<Root> ResolvedName<Root> {
    /// The submitted textual path.
    pub fn path(&self) -> &NamePath<Root> {
        &self.path
    }

    /// Its complete durable chain.
    pub fn encoded_id(&self) -> &EncodedId<Root> {
        &self.encoded_id
    }
}

/// Durable result of one successful seal.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct SealReceipt<Root> {
    pub(crate) operation_key: OperationKey,
    pub(crate) request_digest: RequestDigest,
    pub(crate) changed_tables: Vec<TableChange<Root>>,
    pub(crate) declarations: Vec<ResolvedName<Root>>,
    pub(crate) references: Vec<ResolvedName<Root>>,
    pub(crate) resulting_revision: StateRevision,
}

impl<Root> SealReceipt<Root> {
    /// The idempotency key.
    pub fn operation_key(&self) -> OperationKey {
        self.operation_key
    }

    /// The canonical request digest.
    pub fn request_digest(&self) -> RequestDigest {
        self.request_digest
    }

    /// Every table generation and snapshot created by this seal.
    ///
    /// The records are ordered by table address and remain exact during an
    /// idempotent replay even if those tables have advanced since the seal.
    pub fn changed_tables(&self) -> &[TableChange<Root>] {
        &self.changed_tables
    }

    /// Every declaration result in deterministic parent-before-child order.
    pub fn declarations(&self) -> &[ResolvedName<Root>] {
        &self.declarations
    }

    /// Every lookup-only reference result in canonical path order.
    pub fn references(&self) -> &[ResolvedName<Root>] {
        &self.references
    }

    /// The committed pure-state revision.
    pub fn resulting_revision(&self) -> StateRevision {
        self.resulting_revision
    }
}

/// Durable result of one chain-preserving operational rename.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RenameReceipt<Root> {
    pub(crate) operation_key: OperationKey,
    pub(crate) target: EncodedId<Root>,
    pub(crate) old_spelling: Name,
    pub(crate) new_spelling: Name,
    pub(crate) resulting_generation: TableGeneration,
    pub(crate) resulting_revision: StateRevision,
}

impl<Root> RenameReceipt<Root> {
    /// The idempotency key.
    pub fn operation_key(&self) -> OperationKey {
        self.operation_key
    }

    /// The unchanged complete identity.
    pub fn target(&self) -> &EncodedId<Root> {
        &self.target
    }

    /// The spelling replaced by this operation.
    pub fn old_spelling(&self) -> &Name {
        &self.old_spelling
    }

    /// The replacement exact spelling.
    pub fn new_spelling(&self) -> &Name {
        &self.new_spelling
    }

    /// The new immutable generation of the owning table.
    pub fn resulting_generation(&self) -> TableGeneration {
        self.resulting_generation
    }

    /// The committed pure-state revision.
    pub fn resulting_revision(&self) -> StateRevision {
        self.resulting_revision
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub(crate) enum OperationReceipt<Root> {
    Seal(SealReceipt<Root>),
    Rename(RenameReceipt<Root>),
}
