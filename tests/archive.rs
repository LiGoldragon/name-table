mod common;

use common::{FixtureRoot, authored, declared_id, key, member};
use name_table::{NameTable, NameTableError, RenameRequest, SealRequest};

fn populated() -> (
    NameTable<FixtureRoot>,
    SealRequest<FixtureRoot>,
    name_table::EncodedId<FixtureRoot>,
) {
    let mut table = NameTable::new();
    let request = SealRequest::new(key(50), vec![authored(vec![member("Status")])], vec![]);
    let receipt = table.seal(request.clone()).unwrap();
    let id = declared_id(&receipt, &[], "Status");
    table
        .rename(RenameRequest::new(key(51), id.clone(), "State"))
        .unwrap();
    (table, request, id)
}

#[test]
fn complete_state_and_idempotency_receipts_survive_archive_restart() {
    let (table, original_request, id) = populated();
    let bytes = table.to_archive_bytes().unwrap();
    let mut restored = NameTable::from_archive_bytes(bytes.as_ref()).unwrap();

    assert_eq!(restored, table);
    assert_eq!(restored.resolve(&id).unwrap().as_str(), "State");
    let revision = restored.revision();
    let replay = restored.seal(original_request).unwrap();
    assert_eq!(replay.resulting_revision().value(), 1);
    assert_eq!(restored.revision(), revision);
}

#[test]
fn payload_corruption_is_refused() {
    let (table, _, _) = populated();
    let mut bytes = table.to_archive_bytes().unwrap();
    let final_index = bytes.len() - 1;
    bytes[final_index] ^= 0x80;

    assert!(matches!(
        NameTable::<FixtureRoot>::from_archive_bytes(bytes.as_ref()),
        Err(NameTableError::ArchiveIntegrityMismatch)
            | Err(NameTableError::Deserialize(_))
            | Err(NameTableError::SnapshotIntegrityMismatch { .. })
            | Err(NameTableError::InconsistentTableHead { .. })
    ));
}

#[test]
fn receipt_digest_corruption_is_refused() {
    let (table, original_request, _) = populated();
    let request_digest = table.request_digest(&original_request).unwrap();
    let mut bytes = table.to_archive_bytes().unwrap();
    let offset = bytes
        .windows(request_digest.bytes().len())
        .position(|window| window == request_digest.bytes())
        .expect("the archived receipt contains its request digest");
    bytes[offset] ^= 0x80;

    assert!(matches!(
        NameTable::<FixtureRoot>::from_archive_bytes(bytes.as_ref()),
        Err(NameTableError::ArchiveIntegrityMismatch)
    ));
}

#[test]
fn invalid_envelope_and_legacy_versions_are_typed_failures() {
    assert!(matches!(
        NameTable::<FixtureRoot>::from_archive_bytes(b"not an archive"),
        Err(NameTableError::InvalidArchiveEnvelope)
    ));

    let (table, _, _) = populated();
    for version in [1_u16, 2_u16] {
        let mut bytes = table.to_archive_bytes().unwrap();
        bytes[8..10].copy_from_slice(&version.to_le_bytes());
        assert!(matches!(
            NameTable::<FixtureRoot>::from_archive_bytes(bytes.as_ref()),
            Err(NameTableError::UnsupportedArchiveVersion { found }) if found == version
        ));
    }
}
