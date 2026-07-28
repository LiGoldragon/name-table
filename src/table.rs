//! Generic nested-table state and operations.

use std::collections::{BTreeMap, BTreeSet};

use content_identity::PortableArchive;

use crate::archive::{SnapshotMaterial, typed_digest};
use crate::error::NameTableError;
use crate::request::{
    Declaration, ModuleDeclaration, ModulePath, NamePath, NameReference, RenameRequest, SealRequest,
};
use crate::state::{
    AllocationCursor, ModuleTableHead, ModuleTableSnapshot, OperationKey, OperationReceipt,
    RenameReceipt, RequestDigest, ResolvedName, SealReceipt, SealRequestDomain, SnapshotDigest,
    SnapshotIntegrityDomain, StateRevision, TableGeneration, TableMutability,
};
use crate::transaction::{StagedRename, StagedSeal};
use crate::{EncodedId, Name, TableAddress};

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone)]
struct CanonicalSeal<Root> {
    roots: Vec<CanonicalRoot<Root>>,
    declarations: Vec<CanonicalDeclaration<Root>>,
    references: Vec<NameReference<Root>>,
}

#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
struct CanonicalRoot<Root> {
    root: Root,
    mutability: TableMutability,
}

#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
struct CanonicalDeclaration<Root> {
    root: Root,
    table_path: Vec<Name>,
    spelling: Name,
    child_mutability: Option<TableMutability>,
}

/// Complete generic nested-table state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameTable<Root> {
    pub(crate) revision: StateRevision,
    pub(crate) heads: BTreeMap<TableAddress<Root>, ModuleTableHead<Root>>,
    pub(crate) snapshots: BTreeMap<SnapshotDigest, ModuleTableSnapshot<Root>>,
    pub(crate) receipts: BTreeMap<OperationKey, OperationReceipt<Root>>,
}

impl<Root> Default for NameTable<Root> {
    fn default() -> Self {
        Self {
            revision: StateRevision::default(),
            heads: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }
}

#[allow(private_bounds)]
impl<Root> NameTable<Root>
where
    Root: Clone + Ord,
{
    /// Construct empty state. Root tables are provisioned by their first seal.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current pure-state revision.
    pub fn revision(&self) -> StateRevision {
        self.revision
    }

    /// The current head of one table.
    pub fn head(&self, address: &TableAddress<Root>) -> Option<&ModuleTableHead<Root>> {
        self.heads.get(address)
    }

    /// One immutable historical snapshot by integrity locator.
    pub fn snapshot(&self, digest: SnapshotDigest) -> Option<&ModuleTableSnapshot<Root>> {
        self.snapshots.get(&digest)
    }

    /// The current immutable snapshot of one table.
    pub fn current_snapshot(
        &self,
        address: &TableAddress<Root>,
    ) -> Result<&ModuleTableSnapshot<Root>, NameTableError<Root>> {
        let head = self
            .heads
            .get(address)
            .ok_or_else(|| NameTableError::UnknownTable {
                address: address.clone(),
            })?;
        self.snapshots.get(&head.current_snapshot).ok_or_else(|| {
            NameTableError::InconsistentTableHead {
                address: address.clone(),
            }
        })
    }

    /// Look up an exact spelling in one table without allocating.
    pub fn lookup(
        &self,
        address: &TableAddress<Root>,
        spelling: &Name,
    ) -> Result<Option<EncodedId<Root>>, NameTableError<Root>> {
        Ok(self
            .current_snapshot(address)?
            .lookup_local(spelling)
            .map(|local| address.entry(local)))
    }

    /// Resolve one complete encoded-ID chain through the current snapshot.
    pub fn resolve(&self, encoded_id: &EncodedId<Root>) -> Result<&Name, NameTableError<Root>> {
        let address = encoded_id.owning_table();
        self.current_snapshot(&address)?
            .resolve_local(encoded_id.local())
            .ok_or_else(|| NameTableError::UnknownEncodedId {
                encoded_id: encoded_id.clone(),
            })
    }

    /// Resolve an identity against a specifically pinned historical snapshot.
    pub fn resolve_in_snapshot(
        &self,
        snapshot: SnapshotDigest,
        encoded_id: &EncodedId<Root>,
    ) -> Result<&Name, NameTableError<Root>> {
        let archived = self
            .snapshots
            .get(&snapshot)
            .ok_or_else(|| NameTableError::SnapshotIntegrityMismatch { digest: snapshot })?;
        if archived.address != encoded_id.owning_table() {
            return Err(NameTableError::UnknownEncodedId {
                encoded_id: encoded_id.clone(),
            });
        }
        archived
            .resolve_local(encoded_id.local())
            .ok_or_else(|| NameTableError::UnknownEncodedId {
                encoded_id: encoded_id.clone(),
            })
    }

    /// Compute the digest of a seal's canonical graph.
    ///
    /// Root graphs, declarations in each table, nested modules, and references
    /// are sorted canonically. The operation key is deliberately excluded.
    pub fn request_digest(
        &self,
        request: &SealRequest<Root>,
    ) -> Result<RequestDigest, NameTableError<Root>>
    where
        CanonicalSeal<Root>: PortableArchive,
        <CanonicalSeal<Root> as rkyv::Archive>::Archived:
            rkyv::Deserialize<
                    CanonicalSeal<Root>,
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
        typed_digest::<SealRequestDomain, _>(&canonical_seal(request))
            .map_err(NameTableError::Serialize)
    }

    /// Purely stage one atomic nested seal. Dropping or rolling back the returned
    /// value cannot mutate `self`.
    pub fn stage_seal(
        &self,
        request: SealRequest<Root>,
    ) -> Result<StagedSeal<Root>, NameTableError<Root>>
    where
        CanonicalSeal<Root>: PortableArchive,
        <CanonicalSeal<Root> as rkyv::Archive>::Archived:
            rkyv::Deserialize<
                    CanonicalSeal<Root>,
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
        let request_digest = self.request_digest(&request)?;
        if let Some(existing) = self.receipts.get(&request.operation_key()) {
            return match existing {
                OperationReceipt::Seal(receipt) if receipt.request_digest == request_digest => Ok(
                    StagedSeal::replay(self.revision, self.clone(), receipt.clone()),
                ),
                _ => Err(NameTableError::IdempotencyConflict {
                    operation_key: request.operation_key(),
                }),
            };
        }

        validate_request(&request)?;
        let canonical = canonical_seal(&request);
        let mut staged = self.clone();
        let mut declarations = Vec::new();

        for root in request.roots() {
            let address = TableAddress::root(root.root().clone());
            staged.ensure_table(&address, root.mutability())?;
            staged.apply_declarations(
                &address,
                &ModulePath::new(root.root().clone(), Vec::new()),
                &canonical_declarations(root.declarations()),
                &mut declarations,
            )?;
        }

        let mut references = Vec::with_capacity(canonical.references.len());
        for reference in &canonical.references {
            let encoded_id = staged.resolve_reference(reference)?;
            references.push(ResolvedName {
                path: reference.path().clone(),
                encoded_id,
            });
        }

        let resulting_revision = staged
            .revision
            .next()
            .ok_or(NameTableError::StateRevisionExhausted)?;
        staged.revision = resulting_revision;
        let receipt = SealReceipt {
            operation_key: request.operation_key(),
            request_digest,
            declarations,
            references,
            resulting_revision,
        };
        staged.receipts.insert(
            request.operation_key(),
            OperationReceipt::Seal(receipt.clone()),
        );
        staged.validate()?;
        Ok(StagedSeal::new(self.revision, staged, receipt))
    }

    /// Stage and commit one atomic nested seal.
    pub fn seal(
        &mut self,
        request: SealRequest<Root>,
    ) -> Result<SealReceipt<Root>, NameTableError<Root>>
    where
        CanonicalSeal<Root>: PortableArchive,
        <CanonicalSeal<Root> as rkyv::Archive>::Archived:
            rkyv::Deserialize<
                    CanonicalSeal<Root>,
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
        self.stage_seal(request)?.commit(self)
    }

    /// Purely stage one identity-preserving operational rename.
    pub fn stage_rename(
        &self,
        request: RenameRequest<Root>,
    ) -> Result<StagedRename<Root>, NameTableError<Root>>
    where
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
        if let Some(existing) = self.receipts.get(&request.operation_key()) {
            return match existing {
                OperationReceipt::Rename(receipt)
                    if receipt.target == *request.target()
                        && receipt.new_spelling == *request.new_spelling() =>
                {
                    Ok(StagedRename::replay(
                        self.revision,
                        self.clone(),
                        receipt.clone(),
                    ))
                }
                _ => Err(NameTableError::IdempotencyConflict {
                    operation_key: request.operation_key(),
                }),
            };
        }

        let mut staged = self.clone();
        let address = request.target().owning_table();

        // Mutability is intentionally checked before target entry lookup.
        let head = staged
            .heads
            .get(&address)
            .ok_or_else(|| NameTableError::UnknownTable {
                address: address.clone(),
            })?;
        if head.mutability == TableMutability::Immutable {
            return Err(NameTableError::ImmutableTable { address });
        }

        let current = staged.current_snapshot(&address)?.clone();
        let local = request.target().local();
        let old_spelling = current.resolve_local(local).cloned().ok_or_else(|| {
            NameTableError::UnknownEncodedId {
                encoded_id: request.target().clone(),
            }
        })?;
        if let Some(existing) = current.lookup_local(request.new_spelling())
            && existing != local
        {
            return Err(NameTableError::SpellingCollision {
                address,
                spelling: request.new_spelling().clone(),
                existing,
            });
        }

        let mut entries = current.entries.clone();
        entries[usize::from(local.value())] = request.new_spelling().clone();
        let generation =
            current
                .generation
                .next()
                .ok_or_else(|| NameTableError::TableGenerationExhausted {
                    address: address.clone(),
                })?;
        let snapshot = staged.make_snapshot(&address, generation, entries)?;
        let digest = snapshot.digest;
        staged.snapshots.insert(digest, snapshot);
        let head = staged
            .heads
            .get_mut(&address)
            .expect("owning head was checked before entry lookup");
        head.generation = generation;
        head.current_snapshot = digest;

        let resulting_revision = staged
            .revision
            .next()
            .ok_or(NameTableError::StateRevisionExhausted)?;
        staged.revision = resulting_revision;
        let receipt = RenameReceipt {
            operation_key: request.operation_key(),
            target: request.target().clone(),
            old_spelling,
            new_spelling: request.new_spelling().clone(),
            resulting_generation: generation,
            resulting_revision,
        };
        staged.receipts.insert(
            request.operation_key(),
            OperationReceipt::Rename(receipt.clone()),
        );
        staged.validate()?;
        Ok(StagedRename::new(self.revision, staged, receipt))
    }

    /// Stage and commit one identity-preserving operational rename.
    pub fn rename(
        &mut self,
        request: RenameRequest<Root>,
    ) -> Result<RenameReceipt<Root>, NameTableError<Root>>
    where
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
        self.stage_rename(request)?.commit(self)
    }

    pub(crate) fn validate(&self) -> Result<(), NameTableError<Root>>
    where
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
        for (digest, snapshot) in &self.snapshots {
            if snapshot.entries.len() > usize::from(u16::MAX) + 1 {
                return Err(NameTableError::SnapshotCapacity {
                    address: snapshot.address.clone(),
                });
            }
            let mut names = BTreeSet::new();
            for spelling in &snapshot.entries {
                if !names.insert(spelling) {
                    return Err(NameTableError::DuplicateSnapshotSpelling {
                        address: snapshot.address.clone(),
                        spelling: spelling.clone(),
                    });
                }
            }
            let actual = typed_digest::<SnapshotIntegrityDomain, _>(
                &SnapshotMaterial::from_snapshot(snapshot),
            )
            .map_err(NameTableError::Serialize)?;
            if *digest != snapshot.digest || actual != snapshot.digest {
                return Err(NameTableError::SnapshotIntegrityMismatch { digest: *digest });
            }
        }

        for (address, head) in &self.heads {
            let snapshot = self.snapshots.get(&head.current_snapshot).ok_or_else(|| {
                NameTableError::InconsistentTableHead {
                    address: address.clone(),
                }
            })?;
            if head.address != *address
                || snapshot.address != *address
                || snapshot.generation != head.generation
                || AllocationCursor::for_entry_count(snapshot.entries.len())
                    != Some(head.allocation_cursor)
            {
                return Err(NameTableError::InconsistentTableHead {
                    address: address.clone(),
                });
            }

            if let Some((&local, parent_chain)) = address.module_chain().split_last() {
                let parent =
                    TableAddress::new(address.root_variant().clone(), parent_chain.to_vec());
                let parent_snapshot = self.current_snapshot(&parent).map_err(|_| {
                    NameTableError::InconsistentTableAncestry {
                        address: address.clone(),
                    }
                })?;
                if parent_snapshot.resolve_local(local).is_none() {
                    return Err(NameTableError::InconsistentTableAncestry {
                        address: address.clone(),
                    });
                }
            }
        }

        for (operation_key, receipt) in &self.receipts {
            match receipt {
                OperationReceipt::Seal(receipt) => {
                    if receipt.operation_key != *operation_key
                        || receipt.resulting_revision > self.revision
                    {
                        return Err(NameTableError::InconsistentReceipt {
                            operation_key: *operation_key,
                        });
                    }
                    for resolved in receipt.declarations.iter().chain(receipt.references.iter()) {
                        if resolved.encoded_id.chain().is_empty() {
                            return Err(NameTableError::EmptyStoredEncodedId);
                        }
                        if self.resolve(&resolved.encoded_id).is_err() {
                            return Err(NameTableError::InconsistentReceipt {
                                operation_key: *operation_key,
                            });
                        }
                    }
                }
                OperationReceipt::Rename(receipt) => {
                    if receipt.operation_key != *operation_key
                        || receipt.resulting_revision > self.revision
                        || receipt.target.chain().is_empty()
                    {
                        return Err(NameTableError::InconsistentReceipt {
                            operation_key: *operation_key,
                        });
                    }
                    let address = receipt.target.owning_table();
                    let Some(head) = self.heads.get(&address) else {
                        return Err(NameTableError::InconsistentReceipt {
                            operation_key: *operation_key,
                        });
                    };
                    if receipt.resulting_generation > head.generation
                        || self.resolve(&receipt.target).is_err()
                    {
                        return Err(NameTableError::InconsistentReceipt {
                            operation_key: *operation_key,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn ensure_table(
        &mut self,
        address: &TableAddress<Root>,
        mutability: TableMutability,
    ) -> Result<(), NameTableError<Root>>
    where
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
        if let Some(head) = self.heads.get(address) {
            if head.mutability != mutability {
                return Err(NameTableError::TableMutabilityMismatch {
                    address: address.clone(),
                    existing: head.mutability,
                    submitted: mutability,
                });
            }
            return Ok(());
        }

        let snapshot = self.make_snapshot(address, TableGeneration::ZERO, Vec::new())?;
        let digest = snapshot.digest;
        self.snapshots.insert(digest, snapshot);
        self.heads.insert(
            address.clone(),
            ModuleTableHead {
                address: address.clone(),
                mutability,
                generation: TableGeneration::ZERO,
                allocation_cursor: AllocationCursor::ZERO,
                current_snapshot: digest,
            },
        );
        Ok(())
    }

    fn apply_declarations(
        &mut self,
        address: &TableAddress<Root>,
        table_path: &ModulePath<Root>,
        declarations: &[Declaration],
        resolved: &mut Vec<ResolvedName<Root>>,
    ) -> Result<(), NameTableError<Root>>
    where
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
        let current = self.current_snapshot(address)?.clone();
        let mut entries = current.entries.clone();
        let mut cursor = self
            .heads
            .get(address)
            .expect("current snapshot implies a head")
            .allocation_cursor;
        let mut ids = BTreeMap::new();

        for declaration in declarations {
            let spelling = declaration.name();
            let local = if let Some(local) = current.lookup_local(spelling) {
                local
            } else if let Some((local, next)) = cursor.allocate() {
                // A declaration spelling unseen in this module receives a new
                // encoded ID. Changing authored text therefore creates a new
                // identity and leaves the previous entry allocated. Only the
                // rename operation preserves identity across a spelling change.
                entries.push(spelling.clone());
                cursor = next;
                local
            } else {
                return Err(NameTableError::TableCapacity {
                    address: address.clone(),
                });
            };
            ids.insert(spelling.clone(), local);
            resolved.push(ResolvedName {
                path: NamePath::new(table_path.clone(), spelling.clone()),
                encoded_id: address.entry(local),
            });
        }

        if entries != current.entries {
            let generation = current.generation.next().ok_or_else(|| {
                NameTableError::TableGenerationExhausted {
                    address: address.clone(),
                }
            })?;
            let snapshot = self.make_snapshot(address, generation, entries)?;
            let digest = snapshot.digest;
            self.snapshots.insert(digest, snapshot);
            let head = self
                .heads
                .get_mut(address)
                .expect("current snapshot implies a head");
            head.generation = generation;
            head.allocation_cursor = cursor;
            head.current_snapshot = digest;
        }

        for declaration in declarations {
            if let Declaration::Module(module) = declaration {
                let local = ids[module.name()];
                let child_address = address.child(local);
                self.ensure_table(&child_address, module.mutability())?;
                let mut modules = table_path.modules().to_vec();
                modules.push(module.name().clone());
                self.apply_declarations(
                    &child_address,
                    &ModulePath::new(table_path.root().clone(), modules),
                    module.declarations(),
                    resolved,
                )?;
            }
        }
        Ok(())
    }

    fn resolve_reference(
        &self,
        reference: &NameReference<Root>,
    ) -> Result<EncodedId<Root>, NameTableError<Root>> {
        let path = reference.path();
        let mut address = TableAddress::root(path.table().root().clone());
        for module in path.table().modules() {
            let Some(module_id) = self.lookup(&address, module)? else {
                return Err(NameTableError::UnresolvedReference {
                    reference: reference.clone(),
                });
            };
            address = module_id.child_table();
            if !self.heads.contains_key(&address) {
                return Err(NameTableError::UnresolvedReference {
                    reference: reference.clone(),
                });
            }
        }
        self.lookup(&address, path.spelling())?
            .ok_or_else(|| NameTableError::UnresolvedReference {
                reference: reference.clone(),
            })
    }

    fn make_snapshot(
        &self,
        address: &TableAddress<Root>,
        generation: TableGeneration,
        entries: Vec<Name>,
    ) -> Result<ModuleTableSnapshot<Root>, NameTableError<Root>>
    where
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
        let digest = typed_digest::<SnapshotIntegrityDomain, _>(&SnapshotMaterial::new(
            address, generation, &entries,
        ))
        .map_err(NameTableError::Serialize)?;
        Ok(ModuleTableSnapshot {
            address: address.clone(),
            generation,
            entries,
            digest,
        })
    }
}

fn canonical_seal<Root: Clone + Ord>(request: &SealRequest<Root>) -> CanonicalSeal<Root> {
    let mut roots = request
        .roots()
        .iter()
        .map(|root| CanonicalRoot {
            root: root.root().clone(),
            mutability: root.mutability(),
        })
        .collect::<Vec<_>>();
    roots.sort();

    let mut declarations = Vec::new();
    for root in request.roots() {
        flatten_declarations(root.root(), &[], root.declarations(), &mut declarations);
    }
    declarations.sort();

    let mut references = request.references().to_vec();
    references.sort();
    CanonicalSeal {
        roots,
        declarations,
        references,
    }
}

fn flatten_declarations<Root: Clone>(
    root: &Root,
    table_path: &[Name],
    declarations: &[Declaration],
    flattened: &mut Vec<CanonicalDeclaration<Root>>,
) {
    for declaration in declarations {
        let child_mutability = match declaration {
            Declaration::Member(_) => None,
            Declaration::Module(module) => Some(module.mutability()),
        };
        flattened.push(CanonicalDeclaration {
            root: root.clone(),
            table_path: table_path.to_vec(),
            spelling: declaration.name().clone(),
            child_mutability,
        });
        if let Declaration::Module(module) = declaration {
            let mut child_path = table_path.to_vec();
            child_path.push(module.name().clone());
            flatten_declarations(root, &child_path, module.declarations(), flattened);
        }
    }
}

fn canonical_declarations(declarations: &[Declaration]) -> Vec<Declaration> {
    let mut canonical = declarations
        .iter()
        .map(|declaration| match declaration {
            Declaration::Member(name) => Declaration::Member(name.clone()),
            Declaration::Module(module) => Declaration::Module(ModuleDeclaration::new(
                module.name().clone(),
                module.mutability(),
                canonical_declarations(module.declarations()),
            )),
        })
        .collect::<Vec<_>>();
    canonical.sort_by(|left, right| left.name().as_bytes().cmp(right.name().as_bytes()));
    canonical
}

fn validate_request<Root: Clone + Ord>(
    request: &SealRequest<Root>,
) -> Result<(), NameTableError<Root>> {
    let mut roots = BTreeSet::new();
    for root in request.roots() {
        if !roots.insert(root.root()) {
            return Err(NameTableError::DuplicateRootDeclaration {
                root: root.root().clone(),
            });
        }
        validate_declarations(
            &ModulePath::new(root.root().clone(), Vec::new()),
            root.declarations(),
        )?;
    }
    Ok(())
}

fn validate_declarations<Root: Clone + Ord>(
    table_path: &ModulePath<Root>,
    declarations: &[Declaration],
) -> Result<(), NameTableError<Root>> {
    let mut spellings = BTreeSet::new();
    for declaration in declarations {
        if !spellings.insert(declaration.name()) {
            return Err(NameTableError::Redefinition {
                table: table_path.clone(),
                spelling: declaration.name().clone(),
            });
        }
        if let Declaration::Module(module) = declaration {
            let mut modules = table_path.modules().to_vec();
            modules.push(module.name().clone());
            validate_declarations(
                &ModulePath::new(table_path.root().clone(), modules),
                module.declarations(),
            )?;
        }
    }
    Ok(())
}
