//! `extend_from` builds the one continuous schema-into-logos identifier space:
//! every existing index stays stable, new names append above.

use name_table::{Name, NameTable};

#[test]
fn extension_keeps_every_base_identifier_stable() {
    let mut schema = NameTable::new();
    let sequence = schema.intern(Name::new("CommitSequence"));
    let field = schema.intern(Name::new("Field"));
    let reference = schema.intern(Name::new("TypeReference"));

    let logos = NameTable::extend_from(&schema);

    // Every schema identifier resolves to the exact same name in the extension.
    assert_eq!(
        logos.resolve(sequence).unwrap(),
        schema.resolve(sequence).unwrap()
    );
    assert_eq!(logos.resolve(field).unwrap().as_str(), "Field");
    assert_eq!(logos.resolve(reference).unwrap().as_str(), "TypeReference");
}

#[test]
fn re_interning_a_carried_over_name_returns_its_original_identifier() {
    let mut schema = NameTable::new();
    schema.intern(Name::new("CommitSequence"));
    let field = schema.intern(Name::new("Field"));

    let mut logos = NameTable::extend_from(&schema);
    // A name carried over from schema keeps its exact index in logos.
    assert_eq!(logos.intern(Name::new("Field")), field);
}

#[test]
fn new_extension_names_append_above_the_base() {
    let mut schema = NameTable::new();
    schema.intern(Name::new("CommitSequence"));
    schema.intern(Name::new("Field"));
    schema.intern(Name::new("TypeReference"));

    let mut logos = NameTable::extend_from(&schema);
    let logos_only = logos.intern(Name::new("LogosOnly"));
    assert_eq!(logos_only.value(), 3);
}

#[test]
fn extension_does_not_disturb_the_base() {
    let mut schema = NameTable::new();
    schema.intern(Name::new("CommitSequence"));
    let before = schema.to_archive_bytes().unwrap();

    let mut logos = NameTable::extend_from(&schema);
    let logos_only = logos.intern(Name::new("LogosOnly"));

    assert_eq!(schema.len(), 1);
    assert!(schema.resolve(logos_only).is_err());
    assert_eq!(schema.to_archive_bytes().unwrap().as_ref(), before.as_ref());
}
