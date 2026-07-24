//! Composed slices borrow source identifiers instead of cloning their table.

use name_table::{Identifier, IdentifierNamespace, Name, NameTable, NameTableError};

#[test]
fn composition_keeps_borrowed_schema_identifiers_stable() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let sequence = schema
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let field = schema
        .intern(Name::new("Field"))
        .expect("schema allocation");

    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    assert_eq!(sequence, Identifier::Schema(0));
    assert_eq!(
        logos.resolve(sequence).unwrap(),
        schema.resolve(sequence).unwrap()
    );
    assert_eq!(logos.resolve(field).unwrap().as_str(), "Field");
}

#[test]
fn target_names_allocate_only_in_the_target_home_namespace() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    schema
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let mut logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    let logos_only = logos
        .intern(Name::new("LogosOnly"))
        .expect("Logos allocation");

    assert_eq!(logos_only, Identifier::Logos(0));
    assert_eq!(logos.resolve(logos_only).unwrap().as_str(), "LogosOnly");
    assert!(schema.resolve(logos_only).is_err());
}

#[test]
fn a_borrowed_name_resolves_without_copying_or_reinterning() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let field = schema
        .intern(Name::new("Field"))
        .expect("schema allocation");
    let mut logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    assert_eq!(
        logos.intern(Name::new("Field")).expect("borrowed lookup"),
        field
    );
    assert_eq!(
        logos.len(),
        0,
        "borrowing did not append a copied schema name"
    );
}

#[test]
fn multi_hop_composition_resolves_each_source_namespace() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let schema_name = schema
        .intern(Name::new("SchemaEntry"))
        .expect("schema allocation");

    let mut nomos = NameTable::new(IdentifierNamespace::Nomos)
        .compose(&schema)
        .expect("compose schema into nomos");
    let nomos_name = nomos
        .intern(Name::new("NomosEntry"))
        .expect("nomos allocation");

    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&nomos)
        .expect("compose completed nomos closure into logos");

    assert_eq!(logos.resolve(schema_name).unwrap().as_str(), "SchemaEntry");
    assert_eq!(logos.resolve(nomos_name).unwrap().as_str(), "NomosEntry");
    assert_eq!(logos.lookup(&Name::new("SchemaEntry")), Some(schema_name));
    assert_eq!(logos.lookup(&Name::new("NomosEntry")), Some(nomos_name));
}

#[test]
fn cloning_an_uncomposed_table_seals_the_shared_home_in_both_values() {
    let mut original = NameTable::new(IdentifierNamespace::Schema);
    original
        .intern(Name::new("Entry"))
        .expect("schema allocation before cloning");
    let mut cloned = original.clone();

    // Clone shares the home Arc rather than using copy-on-write, so neither value
    // can diverge by mutating the component-owned namespace.
    assert!(matches!(
        original.intern(Name::new("OriginalOnly")),
        Err(NameTableError::HomeSliceBorrowed {
            operation: "intern a name"
        })
    ));
    assert!(matches!(
        cloned.intern(Name::new("CloneOnly")),
        Err(NameTableError::HomeSliceBorrowed {
            operation: "intern a name"
        })
    ));
}

#[test]
fn cloning_a_composed_table_keeps_borrowed_slices_shared_and_sealed() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let entry = schema
        .intern(Name::new("Entry"))
        .expect("schema allocation");
    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    let cloned = logos.clone();
    assert_eq!(cloned.resolve(entry).unwrap().as_str(), "Entry");
    assert!(matches!(
        schema.intern(Name::new("TooLate")),
        Err(NameTableError::HomeSliceBorrowed {
            operation: "intern a name"
        })
    ));
}

#[test]
fn a_namespace_can_only_be_composed_once() {
    let schema = NameTable::new(IdentifierNamespace::Schema);
    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("first compose");

    assert!(matches!(
        logos.compose(&schema),
        Err(NameTableError::DuplicateNamespace(
            IdentifierNamespace::Schema
        ))
    ));
}

#[test]
fn composition_rejects_an_ambiguous_canonical_name_index() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    schema
        .intern(Name::new("Entry"))
        .expect("schema allocation");
    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    logos.intern(Name::new("Entry")).expect("Logos allocation");

    assert!(matches!(
        logos.compose(&schema),
        Err(NameTableError::NameIndexCollision {
            first: Identifier::Logos(0),
            second: Identifier::Schema(0),
            ..
        })
    ));
}

#[test]
fn composition_rejects_a_namespace_imported_through_another_table() {
    let schema = NameTable::new(IdentifierNamespace::Schema);
    let nomos = NameTable::new(IdentifierNamespace::Nomos)
        .compose(&schema)
        .expect("compose schema into nomos");
    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&nomos)
        .expect("compose nomos closure into logos");

    assert!(matches!(
        logos.compose(&schema),
        Err(NameTableError::DuplicateNamespace(
            IdentifierNamespace::Schema
        ))
    ));
}

#[test]
fn composition_rejects_a_canonical_name_in_a_transitive_import() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    schema
        .intern(Name::new("Entry"))
        .expect("schema allocation");
    let nomos = NameTable::new(IdentifierNamespace::Nomos)
        .compose(&schema)
        .expect("compose schema into nomos");

    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    logos.intern(Name::new("Entry")).expect("logos allocation");

    assert!(matches!(
        logos.compose(&nomos),
        Err(NameTableError::NameIndexCollision {
            first: Identifier::Logos(0),
            second: Identifier::Schema(0),
            ..
        })
    ));
}

#[test]
fn composition_seals_the_source_home_slice_against_mutation() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    schema
        .intern(Name::new("Entry"))
        .expect("schema allocation");
    let _logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    assert!(matches!(
        schema.intern(Name::new("TooLate")),
        Err(NameTableError::HomeSliceBorrowed {
            operation: "intern a name"
        })
    ));
}
