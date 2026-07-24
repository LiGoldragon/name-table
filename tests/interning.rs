//! Interning determinism, namespace-local allocation, and boundary capabilities.

use name_table::{Identifier, IdentifierNamespace, Name, NameInterner, NameResolver, NameTable};

fn intern_through<Interner: NameInterner>(
    interner: &mut Interner,
    name: &str,
) -> Result<Identifier, name_table::NameTableError> {
    interner.intern(Name::new(name))
}

fn resolve_through<Resolver: NameResolver>(resolver: &Resolver, identifier: Identifier) -> String {
    resolver
        .resolve(identifier)
        .expect("identifier resolves")
        .as_str()
        .to_owned()
}

#[test]
fn interning_is_deterministic_within_one_owned_slice() {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    let first = table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let second = table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    assert_eq!(first, second);
    assert_eq!(table.len(), 1);
}

#[test]
fn distinct_names_get_distinct_namespace_local_identifiers() {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    let sequence = table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let field = table.intern(Name::new("Field")).expect("schema allocation");
    assert_eq!(sequence, Identifier::Schema(0));
    assert_eq!(field, Identifier::Schema(1));
}

#[test]
fn equal_locals_in_different_namespaces_are_distinct() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    assert_ne!(
        schema
            .intern(Name::new("Entry"))
            .expect("schema allocation"),
        logos.intern(Name::new("Entry")).expect("Logos allocation")
    );
}

#[test]
fn resolve_round_trips() {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    let sequence = table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let field = table.intern(Name::new("Field")).expect("schema allocation");
    assert_eq!(
        table.resolve(sequence).expect("resolves").as_str(),
        "CommitSequence"
    );
    assert_eq!(table.resolve(field).expect("resolves").as_str(), "Field");
}

#[test]
fn resolving_an_unknown_identifier_errors() {
    let table = NameTable::new(IdentifierNamespace::Schema);
    assert!(table.resolve(Identifier::Schema(0)).is_err());
}

#[test]
fn interning_and_resolving_through_the_boundary_traits() {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    let direct = table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let through = intern_through(&mut table, "CommitSequence").expect("boundary allocation");
    assert_eq!(direct, through);
    assert_eq!(resolve_through(&table, direct), "CommitSequence");
}
