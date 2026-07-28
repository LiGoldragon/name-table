//! Pure staged operations with explicit commit and effect-free rollback.

use crate::{NameTable, NameTableError, RenameReceipt, SealReceipt, StateRevision};

/// A fully validated seal staged against one base revision.
pub struct StagedSeal<Root> {
    base_revision: StateRevision,
    state: NameTable<Root>,
    receipt: SealReceipt<Root>,
}

impl<Root> StagedSeal<Root> {
    pub(crate) fn new(
        base_revision: StateRevision,
        state: NameTable<Root>,
        receipt: SealReceipt<Root>,
    ) -> Self {
        Self {
            base_revision,
            state,
            receipt,
        }
    }

    pub(crate) fn replay(
        base_revision: StateRevision,
        state: NameTable<Root>,
        receipt: SealReceipt<Root>,
    ) -> Self {
        Self::new(base_revision, state, receipt)
    }

    /// The successful receipt before commit.
    pub fn receipt(&self) -> &SealReceipt<Root> {
        &self.receipt
    }

    /// Commit only if the target still has the staged base revision.
    pub fn commit(
        self,
        target: &mut NameTable<Root>,
    ) -> Result<SealReceipt<Root>, NameTableError<Root>> {
        if target.revision != self.base_revision {
            return Err(NameTableError::StaleStage {
                expected: self.base_revision,
                actual: target.revision,
            });
        }
        *target = self.state;
        Ok(self.receipt)
    }

    /// Discard the staged state without touching its base.
    pub fn rollback(self) {}
}

/// A fully validated operational rename staged against one base revision.
pub struct StagedRename<Root> {
    base_revision: StateRevision,
    state: NameTable<Root>,
    receipt: RenameReceipt<Root>,
}

impl<Root> StagedRename<Root> {
    pub(crate) fn new(
        base_revision: StateRevision,
        state: NameTable<Root>,
        receipt: RenameReceipt<Root>,
    ) -> Self {
        Self {
            base_revision,
            state,
            receipt,
        }
    }

    pub(crate) fn replay(
        base_revision: StateRevision,
        state: NameTable<Root>,
        receipt: RenameReceipt<Root>,
    ) -> Self {
        Self::new(base_revision, state, receipt)
    }

    /// The successful receipt before commit.
    pub fn receipt(&self) -> &RenameReceipt<Root> {
        &self.receipt
    }

    /// Commit only if the target still has the staged base revision.
    pub fn commit(
        self,
        target: &mut NameTable<Root>,
    ) -> Result<RenameReceipt<Root>, NameTableError<Root>> {
        if target.revision != self.base_revision {
            return Err(NameTableError::StaleStage {
                expected: self.base_revision,
                actual: target.revision,
            });
        }
        *target = self.state;
        Ok(self.receipt)
    }

    /// Discard the staged state without touching its base.
    pub fn rollback(self) {}
}
