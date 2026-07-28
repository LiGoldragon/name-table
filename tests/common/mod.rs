#![allow(dead_code)]

use name_table::{
    Declaration, ModuleDeclaration, ModulePath, Name, OperationKey, RootDeclaration, SealReceipt,
    TableMutability,
};

#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
pub enum FixtureRoot {
    Authored,
    Vocabulary,
}

#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Ord, PartialEq, PartialOrd,
)]
pub enum UnrelatedRoot {
    Plane(u8),
}

pub fn key(byte: u8) -> OperationKey {
    OperationKey::new([byte; 32])
}

pub fn member(name: &str) -> Declaration {
    Declaration::Member(Name::new(name))
}

pub fn module(
    name: &str,
    mutability: TableMutability,
    declarations: Vec<Declaration>,
) -> Declaration {
    Declaration::Module(ModuleDeclaration::new(name, mutability, declarations))
}

pub fn authored(declarations: Vec<Declaration>) -> RootDeclaration<FixtureRoot> {
    RootDeclaration::new(
        FixtureRoot::Authored,
        TableMutability::Mutable,
        declarations,
    )
}

pub fn vocabulary(declarations: Vec<Declaration>) -> RootDeclaration<FixtureRoot> {
    RootDeclaration::new(
        FixtureRoot::Vocabulary,
        TableMutability::Immutable,
        declarations,
    )
}

pub fn path(modules: &[&str]) -> ModulePath<FixtureRoot> {
    ModulePath::new(
        FixtureRoot::Authored,
        modules.iter().copied().map(Name::new).collect(),
    )
}

pub fn declared_id(
    receipt: &SealReceipt<FixtureRoot>,
    modules: &[&str],
    spelling: &str,
) -> name_table::EncodedId<FixtureRoot> {
    receipt
        .declarations()
        .iter()
        .find(|resolved| {
            resolved.path().table().modules()
                == modules
                    .iter()
                    .copied()
                    .map(Name::new)
                    .collect::<Vec<_>>()
                    .as_slice()
                && resolved.path().spelling().as_str() == spelling
        })
        .expect("declaration result exists")
        .encoded_id()
        .clone()
}
