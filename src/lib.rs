//! Pure facts and capability contracts for opaque name associations.
//!
//! This crate owns no mutable table, entropy source, or journal.  The Sema
//! authority owns those operational concerns.  `EncodedName` is archive and
//! reference data; an authority must mint it from a CSPRNG and must never accept
//! caller-selected bytes for a create operation.

use std::collections::BTreeSet;

use content_identity::ContentAddressedHash;

/// An opaque 128-bit encoded reference.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct EncodedName([u8; 16]);

impl EncodedName {
    /// Reconstruct archived/reference data. This is not an allocation API.
    pub const fn from_archive_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Exact archive/reference bytes.
    pub const fn archive_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// The opaque content identity of one canonical object body.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct TrueName(ContentAddressedHash);

impl TrueName {
    /// Wrap a content identity already derived by the authority from its
    /// validated canonical body projection.
    pub const fn from_content_addressed_hash(hash: ContentAddressedHash) -> Self {
        Self(hash)
    }

    /// The opaque content identity for archive and comparison use.
    pub const fn content_addressed_hash(self) -> ContentAddressedHash {
        self.0
    }
}

/// A monotonic revision that changes only when object content changes.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct ContentRevision(u64);

impl ContentRevision {
    /// The revision assigned at creation.
    pub const INITIAL: Self = Self(0);

    /// Numeric archive value.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// The next content revision, or a typed overflow refusal.
    pub fn next(self) -> Result<Self, NameTableError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(NameTableError::ContentRevisionExhausted)
    }
}

/// Caller-supplied idempotency key for a mutation request.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct OperationKey([u8; 32]);

impl OperationKey {
    /// Construct an exact idempotency key.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Exact visible spelling, kept outside content identity.
#[derive(
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct VisibleName(String);

impl VisibleName {
    /// Record an exact spelling without normalization.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    /// Read the exact spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Structured module placement for a textual projection.
#[derive(
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct ModulePlacement(Vec<String>);

impl ModulePlacement {
    /// Record ordered module segments (empty means root).
    pub fn new(segments: Vec<String>) -> Self {
        Self(segments)
    }
    /// Ordered module segments.
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

/// Structured file placement for a textual projection.
#[derive(
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct FilePlacement(Vec<String>);

impl FilePlacement {
    /// Record ordered file-path segments.
    pub fn new(segments: Vec<String>) -> Self {
        Self(segments)
    }
    /// Ordered file-path segments.
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

/// Textual-only data keyed operationally by one encoded name.
#[derive(
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct TextualMetadata {
    visible_name: VisibleName,
    module_placement: ModulePlacement,
    file_placement: FilePlacement,
}

impl TextualMetadata {
    /// Construct structured textual metadata.
    pub const fn new(
        visible_name: VisibleName,
        module_placement: ModulePlacement,
        file_placement: FilePlacement,
    ) -> Self {
        Self {
            visible_name,
            module_placement,
            file_placement,
        }
    }
    /// Visible spelling.
    pub const fn visible_name(&self) -> &VisibleName {
        &self.visible_name
    }
    /// Module placement.
    pub const fn module_placement(&self) -> &ModulePlacement {
        &self.module_placement
    }
    /// File placement.
    pub const fn file_placement(&self) -> &FilePlacement {
        &self.file_placement
    }
}

/// Whether an issued encoded name remains live.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum NameLifecycle {
    Live,
    Tombstoned,
}

/// One forward association fact.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct NameAssociation {
    true_name: TrueName,
    content_revision: ContentRevision,
    lifecycle: NameLifecycle,
}

impl NameAssociation {
    /// Construct an archiveable association fact.
    pub const fn new(
        true_name: TrueName,
        content_revision: ContentRevision,
        lifecycle: NameLifecycle,
    ) -> Self {
        Self {
            true_name,
            content_revision,
            lifecycle,
        }
    }
    /// Current content identity.
    pub const fn true_name(&self) -> TrueName {
        self.true_name
    }
    /// Current content revision.
    pub const fn content_revision(&self) -> ContentRevision {
        self.content_revision
    }
    /// Current lifecycle.
    pub const fn lifecycle(&self) -> NameLifecycle {
        self.lifecycle
    }
}

/// Immutable realized change fact. Create records the actual random bytes, so
/// replay reads facts and never regenerates entropy or content identities.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct NameChange {
    sequence: u64,
    operation_key: OperationKey,
    action: RealizedNameAction,
}

impl NameChange {
    /// Record one realized action at an exact sequence.
    pub const fn new(
        sequence: u64,
        operation_key: OperationKey,
        action: RealizedNameAction,
    ) -> Self {
        Self {
            sequence,
            operation_key,
            action,
        }
    }
    /// Exact append-only sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Idempotency key.
    pub const fn operation_key(&self) -> OperationKey {
        self.operation_key
    }
    /// Realized facts.
    pub const fn action(&self) -> &RealizedNameAction {
        &self.action
    }
}

/// Realized association mutations.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum RealizedNameAction {
    /// A random encoded name was associated with a true name.
    Create {
        encoded_name: EncodedName,
        true_name: TrueName,
        metadata: TextualMetadata,
    },
    /// Existing name, new content identity and exact next revision.
    ReviseContent {
        encoded_name: EncodedName,
        expected_revision: ContentRevision,
        next_true_name: TrueName,
        next_revision: ContentRevision,
    },
    /// Textual metadata only; semantic identity and revision do not change.
    SetTextualMetadata {
        encoded_name: EncodedName,
        metadata: TextualMetadata,
    },
    /// Permanently tombstone an issued encoded name.
    Delete {
        encoded_name: EncodedName,
        expected_revision: ContentRevision,
    },
}

/// Receipt returned by an idempotent successful mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct NameChangeReceipt {
    operation_key: OperationKey,
    sequence: u64,
    encoded_name: EncodedName,
    content_revision: ContentRevision,
    lifecycle: NameLifecycle,
}

impl NameChangeReceipt {
    /// Construct a realized receipt.
    pub const fn new(
        operation_key: OperationKey,
        sequence: u64,
        encoded_name: EncodedName,
        content_revision: ContentRevision,
        lifecycle: NameLifecycle,
    ) -> Self {
        Self {
            operation_key,
            sequence,
            encoded_name,
            content_revision,
            lifecycle,
        }
    }
    /// Idempotency key.
    pub const fn operation_key(&self) -> OperationKey {
        self.operation_key
    }
    /// Change sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Associated encoded name.
    pub const fn encoded_name(&self) -> EncodedName {
        self.encoded_name
    }
    /// Resulting content revision.
    pub const fn content_revision(&self) -> ContentRevision {
        self.content_revision
    }
    /// Resulting lifecycle.
    pub const fn lifecycle(&self) -> NameLifecycle {
        self.lifecycle
    }
}

/// Read-only association and textual projection contract.
pub trait NameView {
    /// Association including tombstones.
    fn association(&self, encoded_name: &EncodedName) -> Option<&NameAssociation>;
    /// Every live encoded name sharing one true name.
    fn encoded_names(&self, true_name: &TrueName) -> Option<&BTreeSet<EncodedName>>;
    /// Textual metadata for a live name.
    fn textual_metadata(&self, encoded_name: &EncodedName) -> Option<&TextualMetadata>;
}

/// Mutation capability. Its Create operation intentionally takes no
/// `EncodedName`: the implementation owns CSPRNG allocation.
pub trait NameAssociationAuthority: NameView {
    /// Concrete authority refusal type.
    type Refusal;
    /// Stage and commit creation with an authority-minted encoded name.
    fn create(
        &mut self,
        operation_key: OperationKey,
        true_name: TrueName,
        metadata: TextualMetadata,
    ) -> Result<NameChangeReceipt, Self::Refusal>;
    /// Revise semantic content while retaining the encoded name.
    fn revise_content(
        &mut self,
        operation_key: OperationKey,
        encoded_name: EncodedName,
        expected_revision: ContentRevision,
        next_true_name: TrueName,
    ) -> Result<NameChangeReceipt, Self::Refusal>;
    /// Replace textual metadata only.
    fn set_textual_metadata(
        &mut self,
        operation_key: OperationKey,
        encoded_name: EncodedName,
        metadata: TextualMetadata,
    ) -> Result<NameChangeReceipt, Self::Refusal>;
    /// Tombstone an encoded name forever.
    fn delete(
        &mut self,
        operation_key: OperationKey,
        encoded_name: EncodedName,
        expected_revision: ContentRevision,
    ) -> Result<NameChangeReceipt, Self::Refusal>;
}

/// Exact reconstruction capability over a concrete authority-owned snapshot.
pub trait NameChangeReplay {
    /// Concrete snapshot owned by the authority implementation.
    type Snapshot;
    /// Typed replay refusal.
    type Refusal;
    /// Rebuild from a validated snapshot and realized facts only.
    fn replay(
        snapshot: Self::Snapshot,
        changes: &[NameChange],
    ) -> Result<Self::Snapshot, Self::Refusal>;
}

/// Shared pure refusals for malformed facts.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NameTableError {
    #[error("content revision overflowed")]
    ContentRevisionExhausted,
}
