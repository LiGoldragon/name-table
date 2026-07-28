# name-table

Generic nested module-owned name tables for stringless encoded forms.

The crate does not define the production root-table variants. A caller supplies
any portable, ordered root type. Beneath each root, every module owns the exact
spelling table of its immediate members, and the module itself is an entry in
its containing table. Durable identity is a root plus a non-empty chain of
module-local `u16` encoded IDs.

## Contract

- `TableAddress<Root>` addresses a root or module-owned table with a
  zero-or-more local-ID chain.
- `EncodedId<Root>` is a durable identity with a non-empty chain.
- `ModuleTableHead` carries per-table mutability, immutable generation,
  explicit next-or-exhausted allocation state, and the current snapshot
  locator.
- Tables are exact and case-sensitive. Lookup and uniqueness are scoped to one
  module table.
- Declarations allocate. References only resolve.
- One seal validates and stages the complete nested graph, allocates fresh
  spellings in canonical exact-byte order within each table, resolves
  references against committed plus staged declarations, and commits
  everything or nothing.
- Operational rename changes one exact spelling while preserving the complete
  target chain, any child-table address, and every descendant chain.
- Immutable tables refuse rename before target lookup.
- Historical snapshots remain available by typed integrity digest.

Changing authored text is not an operational rename. An unseen replacement
spelling receives a fresh local ID and the old spelling remains allocated and
resolvable. There is no identity-continuation input.

Snapshot digests are integrity and caching metadata. The crate makes no
recursive per-thing content-hashing decision.

## Archive

Version 0.2 uses archive layout 2 for complete nested-table state and durable
idempotency receipts. Layout 1's flat component slices are rejected explicitly;
they cannot be inferred into module chains.

## Build and test

```sh
nix flake check
cargo test
```

The durable gate runs build, tests, documentation, formatting, and clippy.
