//! Portable archive round-trip of one owned namespace slice.

use name_table::{IdentifierNamespace, Name, NameTable, NameTableError};

const SCHEMA_SLICE_LAYOUT_THREE: [u8; 32] = [
    0x6c, 0x5f, 0xc4, 0x98, 0x28, 0x3e, 0x9e, 0x64, 0x82, 0x9e, 0x0e, 0x63, 0x70, 0xd0, 0xba, 0x8a,
    0xbe, 0xb2, 0x80, 0xdc, 0xed, 0x90, 0x1c, 0xb4, 0xe5, 0xf7, 0x42, 0x27, 0xb3, 0xb6, 0xae, 0x79,
];

fn populated() -> NameTable {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    table
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    table.intern(Name::new("Field")).expect("schema allocation");
    table
        .intern(Name::new("TypeReference"))
        .expect("schema allocation");
    table
}

#[test]
fn a_populated_home_slice_round_trips_through_portable_archive() {
    let table = populated();
    let bytes = table.to_archive_bytes().expect("serialize");
    let restored = NameTable::from_archive_bytes(bytes.as_ref()).expect("deserialize");
    assert_eq!(table, restored);
    assert_eq!(restored.namespace(), IdentifierNamespace::Schema);
}

#[test]
fn round_trip_preserves_every_variant_identifier() {
    let table = populated();
    let restored =
        NameTable::from_archive_bytes(table.to_archive_bytes().unwrap().as_ref()).unwrap();
    let final_allocated_local = u16::try_from(table.len() - 1).unwrap();
    for local in 0..=final_allocated_local {
        let identifier = IdentifierNamespace::Schema.identifier(local);
        assert_eq!(
            table.resolve(identifier).unwrap(),
            restored.resolve(identifier).unwrap()
        );
    }
}

#[test]
fn archive_payload_corruption_returns_a_typed_deserialization_error() {
    let bytes = populated().to_archive_bytes().expect("serialize");
    let truncated = &bytes.as_ref()[..bytes.len() - 1];

    assert!(matches!(
        NameTable::from_archive_bytes(truncated),
        Err(NameTableError::Deserialize(_))
    ));
}

#[test]
fn corrupt_archive_envelope_returns_a_typed_error() {
    assert!(matches!(
        NameTable::from_archive_bytes(b"not a name-table archive"),
        Err(NameTableError::InvalidArchiveEnvelope)
    ));
}

#[test]
fn unsupported_archive_version_returns_a_typed_error() {
    let mut bytes = populated().to_archive_bytes().expect("serialize");
    // The current envelope is `NTABLE\\0\\0` followed by a little-endian u16
    // version. Keep this witness at the wire boundary rather than adding a
    // compatibility decoder for the old raw payload.
    bytes[8..10].copy_from_slice(&2_u16.to_le_bytes());

    assert!(matches!(
        NameTable::from_archive_bytes(bytes.as_ref()),
        Err(NameTableError::UnsupportedArchiveVersion { found: 2 })
    ));
}

#[test]
fn legacy_raw_archive_layout_returns_a_typed_envelope_error() {
    let bytes = populated().to_archive_bytes().expect("serialize");
    let raw_payload = &bytes.as_ref()[10..];

    assert!(matches!(
        NameTable::from_archive_bytes(raw_payload),
        Err(NameTableError::InvalidArchiveEnvelope)
    ));
}

#[test]
fn identity_is_stable_across_a_round_trip() {
    let table = populated();
    let restored =
        NameTable::from_archive_bytes(table.to_archive_bytes().unwrap().as_ref()).unwrap();
    assert_eq!(table.identity().unwrap(), restored.identity().unwrap());
    assert_eq!(
        table.identity().unwrap().bytes(),
        &SCHEMA_SLICE_LAYOUT_THREE,
        "the extracted layout-3 slice identity is an absolute compatibility lock"
    );
}

#[test]
fn tables_with_different_names_have_different_identities() {
    let table = populated();
    let mut other = NameTable::new(IdentifierNamespace::Schema);
    other
        .intern(Name::new("CommitSequence"))
        .expect("schema allocation");
    other.intern(Name::new("Field")).expect("schema allocation");
    other
        .intern(Name::new("Renamed"))
        .expect("schema allocation");
    assert_ne!(table.identity().unwrap(), other.identity().unwrap());
}
