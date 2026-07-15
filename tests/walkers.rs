//! Derived-name walkers, verified against the exact behavioral expectations of
//! the two source walkers this crate consolidates.
//!
//! Ported expectations:
//!
//! - `field_name` reproduces `schema`'s `Name::field_name`
//!   (`repos/schema/src/schema.rs:50-65`): an ASCII-uppercase letter opens a
//!   `snake_case` word (underscore before it unless it leads), `-` becomes `_`,
//!   everything else is copied. schema's walker first strips a namespace through
//!   `local_part()`; that namespace split is a schema concern excluded here, so
//!   these fixtures are bare (non-namespaced) names, on which the behavior is
//!   identical.
//! - `screaming` reproduces `schema-rust`'s `ScreamingName::screaming`
//!   (`repos/schema-rust/src/lib.rs:2178-2204`): the same word-boundary walk,
//!   emitting `SCREAMING_SNAKE_CASE`.
//! - `pascal_case` is the inverse round-trip partner (new to this consolidation).

use name_table::Name;

#[test]
fn field_name_matches_schema_field_name() {
    // Exact expectations from schema's PascalCase -> snake_case walker.
    assert_eq!(Name::new("CommitSequence").field_name(), "commit_sequence");
    assert_eq!(
        Name::new("StructDeclaration").field_name(),
        "struct_declaration"
    );
    assert_eq!(Name::new("TypeReference").field_name(), "type_reference");
    assert_eq!(Name::new("SchemaHash").field_name(), "schema_hash");
    assert_eq!(Name::new("Field").field_name(), "field");
    assert_eq!(Name::new("Name").field_name(), "name");
    // The `-` -> `_` branch, shared by both source walkers.
    assert_eq!(Name::new("foo-bar").field_name(), "foo_bar");
}

#[test]
fn screaming_matches_schema_rust_screaming() {
    // Exact expectations from schema-rust's PascalCase -> SCREAMING_SNAKE walker.
    assert_eq!(Name::new("CommitSequence").screaming(), "COMMIT_SEQUENCE");
    assert_eq!(
        Name::new("StructDeclaration").screaming(),
        "STRUCT_DECLARATION"
    );
    assert_eq!(Name::new("TypeReference").screaming(), "TYPE_REFERENCE");
    assert_eq!(Name::new("SchemaHash").screaming(), "SCHEMA_HASH");
    assert_eq!(Name::new("Field").screaming(), "FIELD");
    assert_eq!(Name::new("foo-bar").screaming(), "FOO_BAR");
}

#[test]
fn pascal_case_reconstructs_the_object_spelling() {
    assert_eq!(Name::new("commit_sequence").pascal_case(), "CommitSequence");
    assert_eq!(
        Name::new("struct_declaration").pascal_case(),
        "StructDeclaration"
    );
    assert_eq!(Name::new("foo-bar").pascal_case(), "FooBar");
    assert_eq!(Name::new("name").pascal_case(), "Name");
}

#[test]
fn field_name_and_pascal_case_round_trip() {
    for object in [
        "CommitSequence",
        "StructDeclaration",
        "TypeReference",
        "Field",
    ] {
        let derived_field = Name::new(object).field_name();
        let reconstructed = Name::new(derived_field).pascal_case();
        assert_eq!(reconstructed, object);
    }
}
