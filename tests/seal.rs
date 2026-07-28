mod common;

use common::{FixtureRoot, authored, declared_id, key, member, module, path};
use name_table::{
    Declaration, LocalEncodedId, Name, NameReference, NameTable, NameTableError, RootDeclaration,
    SealRequest, TableAddress, TableMutability,
};

#[test]
fn sibling_modules_may_hold_the_same_spelling_with_different_chains() {
    let mut table = NameTable::new();
    let receipt = table
        .seal(SealRequest::new(
            key(1),
            vec![authored(vec![
                module("billing", TableMutability::Mutable, vec![member("Status")]),
                module("tasks", TableMutability::Mutable, vec![member("Status")]),
            ])],
            vec![],
        ))
        .unwrap();

    let billing = declared_id(&receipt, &["billing"], "Status");
    let tasks = declared_id(&receipt, &["tasks"], "Status");
    assert_ne!(billing, tasks);
    assert_eq!(billing.chain().len(), 2);
    assert_eq!(tasks.chain().len(), 2);
}

#[test]
fn duplicate_exact_spelling_in_one_submitted_module_is_redefinition() {
    let mut table = NameTable::new();
    let before = table.clone();
    let result = table.seal(SealRequest::new(
        key(2),
        vec![authored(vec![member("Status"), member("Status")])],
        vec![],
    ));

    assert!(matches!(
        result,
        Err(NameTableError::Redefinition { spelling, .. })
            if spelling == Name::new("Status")
    ));
    assert_eq!(table, before);
}

#[test]
fn exact_case_variants_are_distinct() {
    let mut table = NameTable::new();
    let receipt = table
        .seal(SealRequest::new(
            key(3),
            vec![authored(vec![member("public"), member("Public")])],
            vec![],
        ))
        .unwrap();

    assert_ne!(
        declared_id(&receipt, &[], "public"),
        declared_id(&receipt, &[], "Public")
    );
}

#[test]
fn unchanged_spelling_in_one_module_preserves_identity() {
    let mut table = NameTable::new();
    let first = table
        .seal(SealRequest::new(
            key(4),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();
    let second = table
        .seal(SealRequest::new(
            key(5),
            vec![authored(vec![member("Status")])],
            vec![],
        ))
        .unwrap();

    assert_eq!(
        declared_id(&first, &[], "Status"),
        declared_id(&second, &[], "Status")
    );
}

#[test]
fn unresolved_references_never_allocate() {
    let mut table = NameTable::new();
    let before = table.clone();
    let reference = NameReference::new(path(&[]), "Missing");
    let result = table.seal(SealRequest::new(
        key(6),
        vec![authored(vec![])],
        vec![reference],
    ));

    assert!(matches!(
        result,
        Err(NameTableError::UnresolvedReference { .. })
    ));
    assert_eq!(table, before);
    assert!(
        table
            .head(&TableAddress::root(FixtureRoot::Authored))
            .is_none()
    );
}

#[test]
fn references_resolve_against_declarations_staged_by_the_same_seal() {
    let mut table = NameTable::new();
    let receipt = table
        .seal(SealRequest::new(
            key(7),
            vec![authored(vec![module(
                "billing",
                TableMutability::Mutable,
                vec![member("Status")],
            )])],
            vec![NameReference::new(path(&["billing"]), "Status")],
        ))
        .unwrap();

    assert_eq!(
        receipt.references()[0].encoded_id(),
        &declared_id(&receipt, &["billing"], "Status")
    );
}

#[test]
fn allocation_and_digest_are_independent_of_traversal_order() {
    let roots_a = vec![authored(vec![
        member("Zulu"),
        module(
            "billing",
            TableMutability::Mutable,
            vec![member("Total"), member("Invoice")],
        ),
        member("Alpha"),
    ])];
    let roots_b = vec![authored(vec![
        member("Alpha"),
        module(
            "billing",
            TableMutability::Mutable,
            vec![member("Invoice"), member("Total")],
        ),
        member("Zulu"),
    ])];
    let references_a = vec![
        NameReference::new(path(&["billing"]), "Total"),
        NameReference::new(path(&[]), "Alpha"),
    ];
    let references_b = vec![
        NameReference::new(path(&[]), "Alpha"),
        NameReference::new(path(&["billing"]), "Total"),
    ];

    let mut first = NameTable::new();
    let mut second = NameTable::new();
    let request_a = SealRequest::new(key(8), roots_a, references_a);
    let request_b = SealRequest::new(key(9), roots_b, references_b);
    let digest_a = first.request_digest(&request_a).unwrap();
    let digest_b = second.request_digest(&request_b).unwrap();
    let receipt_a = first.seal(request_a).unwrap();
    let receipt_b = second.seal(request_b).unwrap();

    assert_eq!(digest_a, digest_b);
    for (modules, spelling) in [
        (&[][..], "Alpha"),
        (&[][..], "Zulu"),
        (&["billing"][..], "Invoice"),
        (&["billing"][..], "Total"),
    ] {
        assert_eq!(
            declared_id(&receipt_a, modules, spelling),
            declared_id(&receipt_b, modules, spelling)
        );
    }
    assert_eq!(
        declared_id(&receipt_a, &[], "Alpha").local(),
        LocalEncodedId::new(0)
    );
    assert_eq!(
        declared_id(&receipt_a, &[], "Zulu").local(),
        LocalEncodedId::new(1)
    );
    assert_eq!(
        declared_id(&receipt_a, &[], "billing").local(),
        LocalEncodedId::new(2)
    );
}

#[test]
fn duplicate_root_graphs_are_refused_without_state() {
    let mut table = NameTable::new();
    let result = table.seal(SealRequest::new(
        key(10),
        vec![
            RootDeclaration::new(
                FixtureRoot::Authored,
                TableMutability::Mutable,
                vec![Declaration::Member(Name::new("A"))],
            ),
            RootDeclaration::new(
                FixtureRoot::Authored,
                TableMutability::Mutable,
                vec![Declaration::Member(Name::new("B"))],
            ),
        ],
        vec![],
    ));
    assert!(matches!(
        result,
        Err(NameTableError::DuplicateRootDeclaration { .. })
    ));
    assert_eq!(table, NameTable::new());
}
