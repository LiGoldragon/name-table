//! Interning determinism, resolve round-trips, and the two boundary capabilities.

use name_table::{Identifier, Name, NameInterner, NameResolver, NameTable};

/// Exercise the mutating decode-side capability generically, proving a codec can
/// be handed only `NameInterner` and still allocate.
fn intern_through<Interner: NameInterner>(interner: &mut Interner, name: &str) -> Identifier {
    interner.intern(Name::new(name))
}

/// Exercise the read-only encode-side capability generically.
fn resolve_through<Resolver: NameResolver>(resolver: &Resolver, identifier: Identifier) -> String {
    resolver
        .resolve(identifier)
        .expect("identifier resolves")
        .as_str()
        .to_owned()
}

#[test]
fn interning_is_deterministic() {
    let mut table = NameTable::new();
    let first = table.intern(Name::new("CommitSequence"));
    let second = table.intern(Name::new("CommitSequence"));
    assert_eq!(first, second);
    assert_eq!(table.len(), 1);
}

#[test]
fn distinct_names_get_distinct_identifiers() {
    let mut table = NameTable::new();
    let sequence = table.intern(Name::new("CommitSequence"));
    let field = table.intern(Name::new("Field"));
    assert_ne!(sequence, field);
    assert_eq!(sequence.value(), 0);
    assert_eq!(field.value(), 1);
}

#[test]
fn resolve_round_trips() {
    let mut table = NameTable::new();
    let sequence = table.intern(Name::new("CommitSequence"));
    let field = table.intern(Name::new("Field"));
    assert_eq!(
        table.resolve(sequence).expect("resolves").as_str(),
        "CommitSequence"
    );
    assert_eq!(table.resolve(field).expect("resolves").as_str(), "Field");
}

#[test]
fn resolving_an_unknown_identifier_errors() {
    let table = NameTable::new();
    assert!(table.resolve(Identifier::new(0)).is_err());
}

#[test]
fn interning_and_resolving_through_the_boundary_traits() {
    let mut table = NameTable::new();
    let direct = table.intern(Name::new("CommitSequence"));
    // Re-interning through the mutating capability returns the same identifier.
    let through = intern_through(&mut table, "CommitSequence");
    assert_eq!(direct, through);
    // Resolving through the read-only capability yields the same name.
    assert_eq!(resolve_through(&table, direct), "CommitSequence");
}
