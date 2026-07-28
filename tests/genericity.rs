mod common;

use common::{FixtureRoot, UnrelatedRoot, authored, key, member};
use name_table::{Declaration, Name, NameTable, RootDeclaration, SealRequest, TableMutability};

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
