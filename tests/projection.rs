//! The Textual-projection surface: a named view is derived from a stringless Core
//! value plus a table, and a rename moves only the projection, never the Core.

use name_table::{Identifier, Name, NameResolver, NameTable, NameTableError, TextualProjection};

/// A stringless toy Core value: a struct declaration carrying identifier indices
/// only — no names. Stands in for the real `Core*` types of later crates.
struct CoreStruct {
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
    type Core = CoreStruct;
    type Textual = TextualStruct;

    fn project<Resolver>(
        core: &CoreStruct,
        names: &Resolver,
    ) -> Result<TextualStruct, NameTableError>
    where
        Resolver: NameResolver,
    {
        let name = names.resolve(core.name)?.as_str().to_owned();
        let mut fields = Vec::with_capacity(core.fields.len());
        for &field in &core.fields {
            fields.push(names.resolve(field)?.as_str().to_owned());
        }
        Ok(TextualStruct { name, fields })
    }
}

#[test]
fn projection_derives_the_named_view_from_the_table() {
    let mut table = NameTable::new();
    let name = table.intern(Name::new("CommitSequence"));
    let author = table.intern(Name::new("Author"));
    let core = CoreStruct {
        name,
        fields: vec![author],
    };

    let view = StructProjection::project(&core, &table).expect("projects");
    assert_eq!(
        view,
        TextualStruct {
            name: "CommitSequence".to_owned(),
            fields: vec!["Author".to_owned()]
        }
    );
}

#[test]
fn a_rename_moves_the_projection_but_not_the_core() {
    // One stringless Core value: indices only.
    let core = CoreStruct {
        name: Identifier::new(0),
        fields: vec![Identifier::new(1)],
    };

    let mut original = NameTable::new();
    original.intern(Name::new("CommitSequence"));
    original.intern(Name::new("Author"));

    // A rename is a table-only edit: identifier 0 now names a different type.
    let mut renamed = NameTable::new();
    renamed.intern(Name::new("CommitLog"));
    renamed.intern(Name::new("Author"));

    let before = StructProjection::project(&core, &original).unwrap();
    let after = StructProjection::project(&core, &renamed).unwrap();

    assert_ne!(before.name, after.name); // the projection moved
    // ...but the Core value carries only indices; nothing about it changed.
    assert_eq!(core.name, Identifier::new(0));
    assert_eq!(core.fields, vec![Identifier::new(1)]);
}

#[test]
fn projecting_a_torn_core_names_the_missing_identifier() {
    let table = NameTable::new();
    let core = CoreStruct {
        name: Identifier::new(7),
        fields: vec![],
    };
    assert!(matches!(
        StructProjection::project(&core, &table),
        Err(NameTableError::UnknownIdentifier(_))
    ));
}
