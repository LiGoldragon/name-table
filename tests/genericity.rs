mod common;

use common::{FixtureRoot, UnrelatedRoot, authored, key, member};
use name_table::{
    Declaration, EmptyEncodedId, EncodedId, LocalEncodedId, Name, NameTable, RootDeclaration,
    SealRequest, TableAddress, TableMutability,
};

#[test]
fn the_same_public_api_accepts_two_unrelated_fixture_root_enums() {
    let mut first: NameTable<FixtureRoot> = NameTable::new();
    first
        .seal(SealRequest::new(
            key(70),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();

    let mut second: NameTable<UnrelatedRoot> = NameTable::new();
    second
        .seal(SealRequest::new(
            key(71),
            vec![RootDeclaration::new(
                UnrelatedRoot::Plane(9),
                TableMutability::Mutable,
                vec![Declaration::Member(Name::new("Status"))],
            )],
            vec![],
        ))
        .unwrap();
}

#[test]
fn table_addresses_allow_empty_chains_but_encoded_ids_do_not() {
    let root = FixtureRoot::Authored;
    let address = TableAddress::new(root.clone(), vec![]);
    assert!(address.is_root());
    assert_eq!(address.module_chain(), &[]);

    assert_eq!(EncodedId::new(root.clone(), vec![]), Err(EmptyEncodedId));

    let local = LocalEncodedId::new(17);
    let encoded = EncodedId::new(root, vec![local]).unwrap();
    assert_eq!(encoded.chain(), &[local]);
    assert_eq!(encoded.owning_table(), address);
}
