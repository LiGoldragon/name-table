//! The Textual-projection surface: a named view is derived from a stringless encoded
//! value plus a table, and a rename moves only the projection, never the encoded value.

use name_table::{
    Identifier, IdentifierNamespace, Name, NameResolver, NameTable, NameTableError,
    TextualProjection,
};

/// A stringless toy encoded value: a struct declaration carrying identifier indices
/// only — no names. Stands in for the real `Encoded*` types of later crates.
struct EncodedStruct {
    name: Identifier,
    fields: Vec<Identifier>,
}

/// The derived named view. In production a concrete `Textual*` type owns this;
/// here it is a test fixture proving the surface is usable.
#[derive(Debug, PartialEq)]
struct TextualStruct {
    name: String,
    fields: Vec<String>,
}

struct StructProjection;

impl TextualProjection for StructProjection {
    type Encoded = EncodedStruct;
    type Textual = TextualStruct;

    fn project<Resolver>(
        encoded: &EncodedStruct,
        names: &Resolver,
    ) -> Result<TextualStruct, NameTableError>
    where
        Resolver: NameResolver,
    {
        let name = names.resolve(encoded.name)?.as_str().to_owned();
        let mut fields = Vec::with_capacity(encoded.fields.len());
        for &field in &encoded.fields {
            fields.push(names.resolve(field)?.as_str().to_owned());
        }
        Ok(TextualStruct { name, fields })
    }
}

#[test]
fn projection_derives_the_named_view_from_the_table() {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    let name = table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    let author = table
        .intern(Name::new("Author"))
        .expect("schema allocation");
    let encoded = EncodedStruct {
        name,
        fields: vec![author],
    };

    let view = StructProjection::project(&encoded, &table).expect("projects");
    assert_eq!(
        view,
        TextualStruct {
            name: "CommitSequence".to_owned(),
            fields: vec!["Author".to_owned()]
        }
    );
}

#[test]
fn a_rename_moves_the_projection_but_not_the_encoded_value() {
    // One stringless encoded value: indices only.
    let encoded = EncodedStruct {
        name: Identifier::Schema(0),
        fields: vec![Identifier::Schema(1)],
    };

    let mut original = NameTable::new(IdentifierNamespace::Schema);
    original
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    original
        .intern(Name::new("Author"))
        .expect("schema allocation");

    // A rename is a table-only edit: identifier 0 now names a different type.
    let mut renamed = NameTable::new(IdentifierNamespace::Schema);
    renamed
        .intern(Name::new("CommitLog"))
        .expect("schema allocation");
    renamed
        .intern(Name::new("Author"))
        .expect("schema allocation");

    let before = StructProjection::project(&encoded, &original).unwrap();
    let after = StructProjection::project(&encoded, &renamed).unwrap();

    assert_ne!(before.name, after.name); // the projection moved
    // ...but the encoded value carries only indices; nothing about it changed.
    assert_eq!(encoded.name, Identifier::Schema(0));
    assert_eq!(encoded.fields, vec![Identifier::Schema(1)]);
}

#[test]
fn projecting_a_torn_encoded_value_names_the_missing_identifier() {
    let table = NameTable::new(IdentifierNamespace::Schema);
    let encoded = EncodedStruct {
        name: Identifier::Schema(7),
        fields: vec![],
    };
    assert!(matches!(
        StructProjection::project(&encoded, &table),
        Err(NameTableError::UnknownIdentifier(_))
    ));
}
