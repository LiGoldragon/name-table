//! Namespace-local capacity and index bounds return typed errors.

use name_table::{Identifier, IdentifierNamespace, Name, NameTable, NameTableError};

#[test]
fn an_out_of_range_identifier_returns_a_typed_error() {
    let table = NameTable::new(IdentifierNamespace::Schema);
    assert!(matches!(
        table.resolve(Identifier::Schema(u16::MAX)),
        Err(NameTableError::UnknownIdentifier(Identifier::Schema(
            u16::MAX
        )))
    ));
}

#[test]
fn a_full_namespace_rejects_the_next_allocation_with_a_typed_error() {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    for local in 0..=u16::MAX {
        let identifier = table
            .intern(Name::new(format!("Name{local}")))
            .expect("local allocation is representable");
        assert_eq!(identifier, Identifier::Schema(local));
    }

    assert!(matches!(
        table.intern(Name::new("OneTooMany")),
        Err(NameTableError::NamespaceCapacity(
            IdentifierNamespace::Schema
        ))
    ));
}
