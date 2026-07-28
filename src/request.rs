//! Typed declaration, reference, and rename requests.

use crate::{EncodedId, Name, OperationKey, TableMutability};

/// One declaration in its owning module table.
///
/// `Module` owns a child table in the submitted graph. The corresponding
/// persisted ownership remains structural: the child table exists at the
/// allocated entry's full chain.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Declaration {
    /// A declaration that introduces no child table in this seal.
    Member(Name),
    /// A declaration that owns a nested module table.
    Module(ModuleDeclaration),
}

impl Declaration {
    /// The exact spelling allocated or reused in the owning table.
    pub fn name(&self) -> &Name {
        match self {
            Self::Member(name) => name,
            Self::Module(module) => &module.name,
        }
    }
}

/// One module declaration and the table it owns.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleDeclaration {
    name: Name,
    mutability: TableMutability,
    declarations: Vec<Declaration>,
}

impl ModuleDeclaration {
    /// Construct one nested module declaration.
    pub fn new(
        name: impl Into<Name>,
        mutability: TableMutability,
        declarations: Vec<Declaration>,
    ) -> Self {
        Self {
            name: name.into(),
            mutability,
            declarations,
        }
    }

    /// The module's exact spelling in its containing table.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// The mutability provisioned on the child table.
    pub fn mutability(&self) -> TableMutability {
        self.mutability
    }

    /// Immediate declarations submitted for the child table.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }
}

/// The submitted declaration graph beneath one root table.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RootDeclaration<Root> {
    root: Root,
    mutability: TableMutability,
    declarations: Vec<Declaration>,
}

impl<Root> RootDeclaration<Root> {
    /// Construct one root-table declaration graph.
    pub fn new(root: Root, mutability: TableMutability, declarations: Vec<Declaration>) -> Self {
        Self {
            root,
            mutability,
            declarations,
        }
    }

    /// The externally supplied root selector.
    pub fn root(&self) -> &Root {
        &self.root
    }

    /// The mutability provisioned on the root table.
    pub fn mutability(&self) -> TableMutability {
        self.mutability
    }

    /// Immediate declarations submitted for the root table.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }
}

/// A module table named textually in a seal request.
///
/// The path is used only at the declaration/reference boundary. Stored state
/// and returned identities use integer chains.
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
pub struct ModulePath<Root> {
    root: Root,
    modules: Vec<Name>,
}

impl<Root> ModulePath<Root> {
    /// Construct a textual module path. An empty path names the root table.
    pub fn new(root: Root, modules: Vec<Name>) -> Self {
        Self { root, modules }
    }

    /// The root-table selector.
    pub fn root(&self) -> &Root {
        &self.root
    }

    /// The module spellings traversed from the root table.
    pub fn modules(&self) -> &[Name] {
        &self.modules
    }
}

/// One exact name addressed textually for declaration results or references.
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
pub struct NamePath<Root> {
    table: ModulePath<Root>,
    spelling: Name,
}

impl<Root> NamePath<Root> {
    /// Construct a name path.
    pub fn new(table: ModulePath<Root>, spelling: impl Into<Name>) -> Self {
        Self {
            table,
            spelling: spelling.into(),
        }
    }

    /// The textually named owning table.
    pub fn table(&self) -> &ModulePath<Root> {
        &self.table
    }

    /// The exact final spelling.
    pub fn spelling(&self) -> &Name {
        &self.spelling
    }
}

/// One lookup-only reference submitted with a seal.
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
pub struct NameReference<Root>(NamePath<Root>);

impl<Root> NameReference<Root> {
    /// Construct a lookup-only reference.
    pub fn new(table: ModulePath<Root>, spelling: impl Into<Name>) -> Self {
        Self(NamePath::new(table, spelling))
    }

    /// The referenced textual path.
    pub fn path(&self) -> &NamePath<Root> {
        &self.0
    }
}

/// One atomic nested seal request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealRequest<Root> {
    operation_key: OperationKey,
    roots: Vec<RootDeclaration<Root>>,
    references: Vec<NameReference<Root>>,
}

impl<Root> SealRequest<Root> {
    /// Construct a seal request.
    pub fn new(
        operation_key: OperationKey,
        roots: Vec<RootDeclaration<Root>>,
        references: Vec<NameReference<Root>>,
    ) -> Self {
        Self {
            operation_key,
            roots,
            references,
        }
    }

    /// The idempotency key.
    pub fn operation_key(&self) -> OperationKey {
        self.operation_key
    }

    /// Submitted root declaration graphs.
    pub fn roots(&self) -> &[RootDeclaration<Root>] {
        &self.roots
    }

    /// Lookup-only references.
    pub fn references(&self) -> &[NameReference<Root>] {
        &self.references
    }
}

/// One identity-preserving spelling edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenameRequest<Root> {
    operation_key: OperationKey,
    target: EncodedId<Root>,
    new_spelling: Name,
}

impl<Root> RenameRequest<Root> {
    /// Construct one operational rename.
    pub fn new(
        operation_key: OperationKey,
        target: EncodedId<Root>,
        new_spelling: impl Into<Name>,
    ) -> Self {
        Self {
            operation_key,
            target,
            new_spelling: new_spelling.into(),
        }
    }

    /// The idempotency key.
    pub fn operation_key(&self) -> OperationKey {
        self.operation_key
    }

    /// The unchanged target identity.
    pub fn target(&self) -> &EncodedId<Root> {
        &self.target
    }

    /// The exact replacement spelling.
    pub fn new_spelling(&self) -> &Name {
        &self.new_spelling
    }
}
