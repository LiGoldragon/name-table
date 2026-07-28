mod common;

use common::{FixtureRoot, authored, declared_id, key, member, module, path};
use name_table::{
    NameReference, NameTable, NameTableError, SealRequest, TableAddress, TableMutability,
};

#[test]
fn dropped_and_explicitly_rolled_back_stages_have_no_effect() {
    let table = NameTable::new();
    let before = table.clone();
    {
        let _staged = table
            .stage_seal(SealRequest::new(
                key(40),
                vec![authored(vec![member("Staged")])],
                vec![],
            ))
            .unwrap();
    }
    assert_eq!(table, before);

    table
        .stage_seal(SealRequest::new(
            key(41),
            vec![authored(vec![member("RolledBack")])],
            vec![],
        ))
        .unwrap()
        .rollback();
    assert_eq!(table, before);
}

#[test]
fn multi_table_failure_rolls_back_every_allocation_and_head() {
    let mut table = NameTable::new();
    let before = table.clone();
    let result = table.seal(SealRequest::new(
        key(42),
        vec![authored(vec![
            module("billing", TableMutability::Mutable, vec![member("Invoice")]),
            module("tasks", TableMutability::Mutable, vec![member("Status")]),
        ])],
        vec![NameReference::new(path(&["billing"]), "Missing")],
    ));

    assert!(matches!(
        result,
        Err(NameTableError::UnresolvedReference { .. })
    ));
    assert_eq!(table, before);
    assert!(
        table
            .head(&TableAddress::root(FixtureRoot::Authored))
            .is_none()
    );
}

#[test]
fn a_stage_refuses_to_commit_over_a_newer_revision() {
    let table = NameTable::new();
    let staged = table
        .stage_seal(SealRequest::new(
            key(43),
            vec![authored(vec![member("First")])],
            vec![],
        ))
        .unwrap();
    let mut advanced = table.clone();
    advanced
        .seal(SealRequest::new(
            key(44),
            vec![authored(vec![member("Second")])],
            vec![],
        ))
        .unwrap();

    assert!(matches!(
        staged.commit(&mut advanced),
        Err(NameTableError::StaleStage { .. })
    ));
}

#[test]
fn identical_idempotent_replay_returns_the_original_receipt_without_advancing() {
    let mut table = NameTable::new();
    let request = SealRequest::new(key(45), vec![authored(vec![member("Status")])], vec![]);
    let first = table.seal(request.clone()).unwrap();
    let revision = table.revision();
    let replay = table.seal(request).unwrap();

    assert_eq!(replay, first);
    assert_eq!(table.revision(), revision);
}

#[test]
fn replay_retains_the_original_changed_table_snapshots() {
    let mut table = NameTable::new();
    let request = SealRequest::new(key(47), vec![authored(vec![member("Status")])], vec![]);
    let first = table.seal(request.clone()).unwrap();
    let status = declared_id(&first, &[], "Status");
    let changed = first
        .changed_tables()
        .iter()
        .find(|change| change.address() == &status.owning_table())
        .expect("the first seal changed the root table");

    table
        .seal(SealRequest::new(
            key(48),
            vec![authored(vec![member("State")])],
            vec![],
        ))
        .unwrap();
    assert_ne!(
        changed.snapshot(),
        table.head(changed.address()).unwrap().current_snapshot()
    );

    let revision = table.revision();
    let replay = table.seal(request).unwrap();
    assert_eq!(replay, first);
    assert_eq!(replay.changed_tables(), first.changed_tables());
    assert_eq!(table.revision(), revision);
    assert_eq!(
        table
            .snapshot(changed.snapshot())
            .unwrap()
            .resolve_local(status.local())
            .unwrap()
            .as_str(),
        "Status"
    );
}

#[test]
fn idempotency_key_reuse_with_different_content_is_refused_without_effect() {
    let mut table = NameTable::new();
    table
        .seal(SealRequest::new(
            key(46),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();
    let before = table.clone();

    assert!(matches!(
        table.seal(SealRequest::new(
            key(46),
            vec![authored(vec![member("State")])],
            vec![],
        )),
        Err(NameTableError::IdempotencyConflict { .. })
    ));
    assert_eq!(table, before);
}
