mod common;

use common::{FixtureRoot, authored, declared_id, key, member, module, vocabulary};
use name_table::{
    EncodedId, LocalEncodedId, Name, NameTable, NameTableError, RenameRequest, SealRequest,
    TableAddress, TableMutability,
};

#[test]
fn text_edit_member_rename_allocates_fresh_and_keeps_old_resolvable() {
    let mut table = NameTable::new();
    let first = table
        .seal(SealRequest::new(
            key(20),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();
    let old = declared_id(&first, &[], "Status");
    let second = table
        .seal(SealRequest::new(
            key(21),
            vec![authored(vec![member("State")])],
            vec![],
        ))
        .unwrap();
    let fresh = declared_id(&second, &[], "State");

    assert_ne!(old, fresh);
    assert_eq!(table.resolve(&old).unwrap().as_str(), "Status");
    assert_eq!(table.resolve(&fresh).unwrap().as_str(), "State");

    let reintroduced = table
        .seal(SealRequest::new(
            key(32),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();
    assert_eq!(declared_id(&reintroduced, &[], "Status"), old);
}

#[test]
fn textual_module_rename_remints_the_descendant_subtree() {
    let mut table = NameTable::new();
    let first = table
        .seal(SealRequest::new(
            key(22),
            vec![authored(vec![module(
                "billing",
                TableMutability::Mutable,
                vec![member("Status")],
            )])],
            vec![],
        ))
        .unwrap();
    let old_module = declared_id(&first, &[], "billing");
    let old_status = declared_id(&first, &["billing"], "Status");

    let second = table
        .seal(SealRequest::new(
            key(23),
            vec![authored(vec![module(
                "ledger",
                TableMutability::Mutable,
                vec![member("Status")],
            )])],
            vec![],
        ))
        .unwrap();
    let new_module = declared_id(&second, &[], "ledger");
    let new_status = declared_id(&second, &["ledger"], "Status");

    assert_ne!(old_module, new_module);
    assert_ne!(old_status, new_status);
    assert_eq!(table.resolve(&old_module).unwrap().as_str(), "billing");
    assert_eq!(table.resolve(&old_status).unwrap().as_str(), "Status");
}

#[test]
fn operational_member_rename_preserves_the_chain_and_history() {
    let mut table = NameTable::new();
    let initial = table
        .seal(SealRequest::new(
            key(24),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();
    let target = declared_id(&initial, &[], "Status");
    let address = target.owning_table();
    let old_snapshot = table.head(&address).unwrap().current_snapshot();

    let receipt = table
        .rename(RenameRequest::new(key(25), target.clone(), "State"))
        .unwrap();

    assert_eq!(receipt.target(), &target);
    assert_eq!(table.resolve(&target).unwrap().as_str(), "State");
    assert_eq!(
        table
            .resolve_in_snapshot(old_snapshot, &target)
            .unwrap()
            .as_str(),
        "Status"
    );
}

#[test]
fn operational_module_rename_preserves_module_and_descendant_chains() {
    let mut table = NameTable::new();
    let initial = table
        .seal(SealRequest::new(
            key(26),
            vec![authored(vec![module(
                "billing",
                TableMutability::Mutable,
                vec![member("Status")],
            )])],
            vec![],
        ))
        .unwrap();
    let module_id = declared_id(&initial, &[], "billing");
    let status_id = declared_id(&initial, &["billing"], "Status");
    let child_address = module_id.child_table();

    table
        .rename(RenameRequest::new(key(27), module_id.clone(), "ledger"))
        .unwrap();

    assert_eq!(table.resolve(&module_id).unwrap().as_str(), "ledger");
    assert_eq!(table.resolve(&status_id).unwrap().as_str(), "Status");
    assert!(table.head(&child_address).is_some());
}

#[test]
fn immutable_table_failure_precedes_unknown_entry_lookup() {
    let mut table = NameTable::new();
    table
        .seal(SealRequest::new(
            key(28),
            vec![vocabulary(vec![member("struct")])],
            vec![],
        ))
        .unwrap();
    let unknown =
        EncodedId::new(FixtureRoot::Vocabulary, vec![LocalEncodedId::new(u16::MAX)]).unwrap();

    assert!(matches!(
        table.rename(RenameRequest::new(key(29), unknown, "record")),
        Err(NameTableError::ImmutableTable { address })
            if address == TableAddress::root(FixtureRoot::Vocabulary)
    ));
}

#[test]
fn operational_rename_refuses_an_existing_same_table_spelling() {
    let mut table = NameTable::new();
    let initial = table
        .seal(SealRequest::new(
            key(30),
            vec![authored(vec![member("Status"), member("State")])],
            vec![],
        ))
        .unwrap();
    let status = declared_id(&initial, &[], "Status");
    let before = table.clone();

    assert!(matches!(
        table.rename(RenameRequest::new(key(31), status, Name::new("State"))),
        Err(NameTableError::SpellingCollision { .. })
    ));
    assert_eq!(table, before);
}

#[test]
fn identical_operational_rename_replays_without_advancing_state() {
    let mut table = NameTable::new();
    let initial = table
        .seal(SealRequest::new(
            key(33),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();
    let target = declared_id(&initial, &[], "Status");
    let request = RenameRequest::new(key(34), target, "State");
    let first = table.rename(request.clone()).unwrap();
    let revision = table.revision();
    let replay = table.rename(request).unwrap();

    assert_eq!(replay, first);
    assert_eq!(table.revision(), revision);
}
