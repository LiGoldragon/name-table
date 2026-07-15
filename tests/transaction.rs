//! Transactional interning: a rolled-back or failed alternative leaves the table
//! observably identical (down to its archived bytes and identity); a committed
//! one merges its staged names at their staged identifiers.

use name_table::{Identifier, Name, NameTable};

fn populated() -> NameTable {
    let mut table = NameTable::new();
    table.intern(Name::new("CommitSequence"));
    table.intern(Name::new("Field"));
    table
}

#[test]
fn a_dropped_transaction_leaves_the_table_byte_identical() {
    let mut table = populated();
    let before_bytes = table.to_archive_bytes().unwrap();
    let before_identity = table.identity().unwrap();
    let before_len = table.len();

    {
        let mut transaction = table.begin();
        transaction.intern(Name::new("Speculative"));
        transaction.intern(Name::new("AnotherOne"));
        // Dropped without commit: an implicit, effect-free rollback.
    }

    assert_eq!(table.len(), before_len);
    assert_eq!(
        table.to_archive_bytes().unwrap().as_ref(),
        before_bytes.as_ref()
    );
    assert_eq!(table.identity().unwrap(), before_identity);
}

#[test]
fn an_explicit_rollback_leaves_the_table_byte_identical() {
    let mut table = populated();
    let before_bytes = table.to_archive_bytes().unwrap();

    let mut transaction = table.begin();
    transaction.intern(Name::new("Speculative"));
    transaction.rollback();

    assert_eq!(
        table.to_archive_bytes().unwrap().as_ref(),
        before_bytes.as_ref()
    );
}

#[test]
fn a_commit_merges_staged_names_at_their_staged_identifiers() {
    let mut table = populated();
    let before_len = table.len();

    let mut transaction = table.begin();
    let staged = transaction.intern(Name::new("Committed"));
    // The staged identifier occupies the index above the committed table.
    assert_eq!(staged.value(), before_len as u32);
    transaction.commit();

    assert_eq!(table.len(), before_len + 1);
    assert_eq!(table.resolve(staged).unwrap().as_str(), "Committed");
}

#[test]
fn a_committed_name_dedups_inside_a_transaction_without_staging() {
    let mut table = populated();
    let committed = table.intern(Name::new("Field"));

    let mut transaction = table.begin();
    let again = transaction.intern(Name::new("Field"));
    assert_eq!(again, committed);
    assert_eq!(transaction.staged_count(), 0);
}

#[test]
fn a_staged_identifier_resolves_within_the_transaction() {
    let mut table = populated();
    let mut transaction = table.begin();
    let staged = transaction.intern(Name::new("Speculative"));
    assert_eq!(transaction.resolve(staged).unwrap().as_str(), "Speculative");
}

#[test]
fn try_intern_rolls_back_a_failed_alternative() {
    let mut table = populated();
    let before_bytes = table.to_archive_bytes().unwrap();

    let outcome: Result<(), &str> = table.try_intern(|transaction| {
        transaction.intern(Name::new("Doomed"));
        Err("decode alternative failed")
    });

    assert!(outcome.is_err());
    // No allocation effect: the interning-atomicity law.
    assert_eq!(
        table.to_archive_bytes().unwrap().as_ref(),
        before_bytes.as_ref()
    );
}

#[test]
fn try_intern_commits_a_successful_alternative() {
    let mut table = populated();

    let outcome: Result<Identifier, &str> =
        table.try_intern(|transaction| Ok(transaction.intern(Name::new("Kept"))));

    let identifier = outcome.expect("alternative succeeded");
    assert_eq!(table.resolve(identifier).unwrap().as_str(), "Kept");
}
