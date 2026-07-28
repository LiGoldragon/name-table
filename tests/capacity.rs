mod common;

use common::{FixtureRoot, authored, key, member};
use name_table::{AllocationCursor, Name, NameTable, NameTableError, SealRequest, TableAddress};

#[test]
fn every_u16_local_is_allocated_before_explicit_exhaustion() {
    let declarations = (0..=u16::MAX)
        .map(|local| member(&format!("Name{local:05}")))
        .collect();
    let mut table = NameTable::new();
    table
        .seal(SealRequest::new(
            key(60),
            vec![authored(declarations)],
            vec![],
        ))
        .unwrap();

    let root = TableAddress::root(FixtureRoot::Authored);
    assert_eq!(
        table.head(&root).unwrap().allocation_cursor(),
        AllocationCursor::Exhausted
    );
    assert_eq!(
        table.current_snapshot(&root).unwrap().entries().len(),
        usize::from(u16::MAX) + 1
    );
    assert_eq!(
        table
            .lookup(&root, &Name::new("Name65535"))
            .unwrap()
            .unwrap()
            .local()
            .value(),
        u16::MAX
    );

    let before = table.clone();
    assert!(matches!(
        table.seal(SealRequest::new(
            key(61),
            vec![authored(vec![member("OneTooMany")])],
            vec![],
        )),
        Err(NameTableError::TableCapacity { address }) if address == root
    ));
    assert_eq!(table, before);
}
