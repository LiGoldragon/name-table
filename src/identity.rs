//! Root-fronted nested encoded identities.

use crate::error::EmptyEncodedId;

/// One table-local encoded ID.
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
pub struct LocalEncodedId(u16);

impl LocalEncodedId {
    /// Construct a local encoded ID from its complete `u16` range.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// The table-local integer.
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// The structural address of one module-owned table.
///
/// An empty module chain addresses the root table selected by `root`. A
/// non-empty chain addresses the child table owned by that complete encoded ID.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct TableAddress<Root> {
    root: Root,
    module_chain: Vec<LocalEncodedId>,
}

impl<Root> TableAddress<Root> {
    /// Address a root table.
    pub fn root(root: Root) -> Self {
        Self {
            root,
            module_chain: Vec::new(),
        }
    }

    /// Construct an address from a root and zero or more owning-module IDs.
    pub fn new(root: Root, module_chain: Vec<LocalEncodedId>) -> Self {
        Self { root, module_chain }
    }

    /// The root-table selector.
    pub fn root_variant(&self) -> &Root {
        &self.root
    }

    /// The zero-or-more owning-module chain.
    pub fn module_chain(&self) -> &[LocalEncodedId] {
        &self.module_chain
    }

    /// Whether this is a root table.
    pub fn is_root(&self) -> bool {
        self.module_chain.is_empty()
    }
}

impl<Root: Clone> TableAddress<Root> {
    /// Address the child table structurally owned by `local` in this table.
    pub fn child(&self, local: LocalEncodedId) -> Self {
        let mut module_chain = self.module_chain.clone();
        module_chain.push(local);
        Self {
            root: self.root.clone(),
            module_chain,
        }
    }

    /// The encoded ID of one entry in this table.
    pub fn entry(&self, local: LocalEncodedId) -> EncodedId<Root> {
        EncodedId {
            root: self.root.clone(),
            chain: self.child(local).module_chain,
        }
    }
}

/// A durable root-fronted, non-empty encoded-ID chain.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct EncodedId<Root> {
    root: Root,
    chain: Vec<LocalEncodedId>,
}

impl<Root> EncodedId<Root> {
    /// Construct a durable identity, rejecting the empty chain reserved for a
    /// table address.
    pub fn new(root: Root, chain: Vec<LocalEncodedId>) -> Result<Self, EmptyEncodedId> {
        if chain.is_empty() {
            return Err(EmptyEncodedId);
        }
        Ok(Self { root, chain })
    }

    /// The root-table selector.
    pub fn root_variant(&self) -> &Root {
        &self.root
    }

    /// The complete non-empty chain.
    pub fn chain(&self) -> &[LocalEncodedId] {
        &self.chain
    }

    /// The final table-local ID.
    pub fn local(&self) -> LocalEncodedId {
        *self
            .chain
            .last()
            .expect("EncodedId construction and archive validation keep the chain non-empty")
    }
}

impl<Root: Clone> EncodedId<Root> {
    /// The table containing the final entry.
    pub fn owning_table(&self) -> TableAddress<Root> {
        let mut module_chain = self.chain.clone();
        module_chain.pop();
        TableAddress::new(self.root.clone(), module_chain)
    }

    /// The child table structurally owned by this identity.
    pub fn child_table(&self) -> TableAddress<Root> {
        TableAddress::new(self.root.clone(), self.chain.clone())
    }
}
