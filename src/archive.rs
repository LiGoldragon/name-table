//! Versioned portable archive and typed integrity hashing.

use std::collections::BTreeMap;

use content_identity::{
    DomainSeparation, HashDomain, IntegrityDigest, IntegrityEnvelope, LayoutVersion,
    PortableArchive,
};

use crate::error::NameTableError;
use crate::state::{
    ModuleTableHead, ModuleTableSnapshot, OperationKey, OperationReceipt, StateRevision,
    TableGeneration,
};
use crate::{Name, NameTable, TableAddress};

/// The hash material of one immutable snapshot.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone)]
pub(crate) struct SnapshotMaterial<Root> {
    address: TableAddress<Root>,
    generation: TableGeneration,
    entries: Vec<Name>,
}

impl<Root: Clone> SnapshotMaterial<Root> {
    pub(crate) fn from_snapshot(snapshot: &ModuleTableSnapshot<Root>) -> Self {
        Self {
            address: snapshot.address.clone(),
            generation: snapshot.generation,
            entries: snapshot.entries.clone(),
        }
    }

    pub(crate) fn new(
        address: &TableAddress<Root>,
        generation: TableGeneration,
        entries: &[Name],
    ) -> Self {
        Self {
            address: address.clone(),
            generation,
            entries: entries.to_vec(),
        }
    }
}

pub(crate) fn typed_digest<Domain, Value>(value: &Value) -> Result<IntegrityDigest<Domain>, String>
where
    Domain: HashDomain,
    Value: PortableArchive,
    Value::Archived: rkyv::Deserialize<Value, rkyv::api::high::HighDeserializer<rkyv::rancor::Error>>
        + for<'validation> rkyv::bytecheck::CheckBytes<
            rkyv::rancor::Strategy<
                rkyv::validation::Validator<
                    rkyv::validation::archive::ArchiveValidator<'validation>,
                    rkyv::validation::shared::SharedValidator,
                >,
                rkyv::rancor::Error,
            >,
        >,
{
    IntegrityDigest::<Domain>::of_core(value).map_err(|error| error.to_string())
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone)]
struct ArchiveState<Root> {
    revision: StateRevision,
    heads: Vec<ModuleTableHead<Root>>,
    snapshots: Vec<ModuleTableSnapshot<Root>>,
    receipts: Vec<(OperationKey, OperationReceipt<Root>)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ArchiveVersion(u16);

impl ArchiveVersion {
    const CURRENT: Self = Self(3);
}

struct ArchiveIntegrityDomain;

impl HashDomain for ArchiveIntegrityDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "name-table archive envelope integrity",
            layout: LayoutVersion::new(1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ArchiveEnvelope {
    version: ArchiveVersion,
}

impl ArchiveEnvelope {
    const MAGIC: [u8; 8] = *b"NTABLE\0\0";
    const VERSIONED_HEADER_LEN: usize = Self::MAGIC.len() + std::mem::size_of::<u16>();
    const HEADER_LEN: usize = Self::VERSIONED_HEADER_LEN + 32;

    const fn current() -> Self {
        Self {
            version: ArchiveVersion::CURRENT,
        }
    }

    fn seal(self, payload: &[u8]) -> rkyv::util::AlignedVec {
        let mut canonical_payload = rkyv::util::AlignedVec::with_capacity(payload.len());
        canonical_payload.extend_from_slice(payload);
        let integrity = IntegrityEnvelope::<ArchiveIntegrityDomain>::seal(canonical_payload);
        let mut bytes = rkyv::util::AlignedVec::with_capacity(Self::HEADER_LEN + payload.len());
        bytes.extend_from_slice(&Self::MAGIC);
        bytes.extend_from_slice(&self.version.0.to_le_bytes());
        bytes.extend_from_slice(integrity.integrity().bytes());
        bytes.extend_from_slice(integrity.payload());
        bytes
    }

    fn open(bytes: &[u8]) -> Result<&[u8], ArchiveEnvelopeError> {
        if bytes.len() < Self::VERSIONED_HEADER_LEN || bytes[..Self::MAGIC.len()] != Self::MAGIC {
            return Err(ArchiveEnvelopeError::Invalid);
        }
        let version = u16::from_le_bytes(
            bytes[Self::MAGIC.len()..Self::VERSIONED_HEADER_LEN]
                .try_into()
                .map_err(|_| ArchiveEnvelopeError::Invalid)?,
        );
        if version != ArchiveVersion::CURRENT.0 {
            return Err(ArchiveEnvelopeError::Unsupported(version));
        }
        if bytes.len() < Self::HEADER_LEN {
            return Err(ArchiveEnvelopeError::Invalid);
        }
        let expected = IntegrityDigest::<ArchiveIntegrityDomain>::from_bytes(
            bytes[Self::VERSIONED_HEADER_LEN..Self::HEADER_LEN]
                .try_into()
                .map_err(|_| ArchiveEnvelopeError::Invalid)?,
        );
        let payload = &bytes[Self::HEADER_LEN..];
        if expected != IntegrityDigest::<ArchiveIntegrityDomain>::derive(payload) {
            return Err(ArchiveEnvelopeError::Integrity);
        }
        Ok(payload)
    }
}

enum ArchiveEnvelopeError {
    Invalid,
    Integrity,
    Unsupported(u16),
}

#[allow(private_bounds)]
impl<Root> NameTable<Root>
where
    Root: Clone + Ord,
{
    /// Serialize the complete generic nested-table state under archive layout 3.
    pub fn to_archive_bytes(&self) -> Result<rkyv::util::AlignedVec, NameTableError<Root>>
    where
        ArchiveState<Root>: PortableArchive,
        <ArchiveState<Root> as rkyv::Archive>::Archived:
            rkyv::Deserialize<
                    ArchiveState<Root>,
                    rkyv::api::high::HighDeserializer<rkyv::rancor::Error>,
                > + for<'validation> rkyv::bytecheck::CheckBytes<
                    rkyv::rancor::Strategy<
                        rkyv::validation::Validator<
                            rkyv::validation::archive::ArchiveValidator<'validation>,
                            rkyv::validation::shared::SharedValidator,
                        >,
                        rkyv::rancor::Error,
                    >,
                >,
    {
        let state = ArchiveState {
            revision: self.revision,
            heads: self.heads.values().cloned().collect(),
            snapshots: self.snapshots.values().cloned().collect(),
            receipts: self
                .receipts
                .iter()
                .map(|(key, receipt)| (*key, receipt.clone()))
                .collect(),
        };
        let payload = state
            .to_archive_bytes()
            .map_err(|error| NameTableError::Serialize(error.to_string()))?;
        Ok(ArchiveEnvelope::current().seal(payload.as_ref()))
    }

    /// Reconstruct and validate complete state from archive layout 3.
    ///
    /// Legacy flat archives carry version 1. The incomplete nested archive
    /// carries version 2. Both are rejected explicitly.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, NameTableError<Root>>
    where
        ArchiveState<Root>: PortableArchive,
        <ArchiveState<Root> as rkyv::Archive>::Archived:
            rkyv::Deserialize<
                    ArchiveState<Root>,
                    rkyv::api::high::HighDeserializer<rkyv::rancor::Error>,
                > + for<'validation> rkyv::bytecheck::CheckBytes<
                    rkyv::rancor::Strategy<
                        rkyv::validation::Validator<
                            rkyv::validation::archive::ArchiveValidator<'validation>,
                            rkyv::validation::shared::SharedValidator,
                        >,
                        rkyv::rancor::Error,
                    >,
                >,
        SnapshotMaterial<Root>: PortableArchive,
        <SnapshotMaterial<Root> as rkyv::Archive>::Archived:
            rkyv::Deserialize<
                    SnapshotMaterial<Root>,
                    rkyv::api::high::HighDeserializer<rkyv::rancor::Error>,
                > + for<'validation> rkyv::bytecheck::CheckBytes<
                    rkyv::rancor::Strategy<
                        rkyv::validation::Validator<
                            rkyv::validation::archive::ArchiveValidator<'validation>,
                            rkyv::validation::shared::SharedValidator,
                        >,
                        rkyv::rancor::Error,
                    >,
                >,
    {
        let payload = ArchiveEnvelope::open(bytes).map_err(|error| match error {
            ArchiveEnvelopeError::Invalid => NameTableError::InvalidArchiveEnvelope,
            ArchiveEnvelopeError::Integrity => NameTableError::ArchiveIntegrityMismatch,
            ArchiveEnvelopeError::Unsupported(found) => {
                NameTableError::UnsupportedArchiveVersion { found }
            }
        })?;
        let state = ArchiveState::<Root>::from_archive_bytes(payload)
            .map_err(|error| NameTableError::Deserialize(error.to_string()))?;

        let mut heads = BTreeMap::new();
        for head in state.heads {
            if heads.insert(head.address.clone(), head).is_some() {
                return Err(NameTableError::DuplicateArchiveRecord);
            }
        }

        let mut snapshots = BTreeMap::new();
        for snapshot in state.snapshots {
            if snapshots.insert(snapshot.digest, snapshot).is_some() {
                return Err(NameTableError::DuplicateArchiveRecord);
            }
        }

        let mut receipts = BTreeMap::new();
        for (key, receipt) in state.receipts {
            if receipts.insert(key, receipt).is_some() {
                return Err(NameTableError::DuplicateArchiveRecord);
            }
        }

        let table = Self {
            revision: state.revision,
            heads,
            snapshots,
            receipts,
        };
        table.validate()?;
        Ok(table)
    }
}
