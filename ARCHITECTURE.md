# name-table architecture

## Status boundary

`name-table` is the string/encodedID correspondence library in the shared
language substrate. Version 0.3 implements the generic nested-table foundation.
Consumers pinned to revisions before 0.2 still see the legacy flat
component-slice API until the coordinated breaking train repins them.

## Nested identity

The implementation is generic over a root-table variant type whose production
variants remain a separate design question. Under one variant, every module
owns the exact spelling table of its immediate members. A module is itself an
entry in its containing module's table.

```text
root table
  1 <-> "billing"
  2 <-> "tasks"

table owned by root/1       table owned by root/2
  1 <-> "Status"              1 <-> "Status"
```

The two `Status` declarations do not collide. Their durable identities are the
complete encodedID chains `root/1/1` and `root/2/1`. Encoded forms carry those
integer chains, never spellings and never a separate declared-thing identity.

The root variant identifies the root table. It does not license this library to
invent the production variant set or attach semantics to it.

## Table state

Each module table has a top-level head containing:

- its structural address: root variant plus the owning-module encodedID chain;
- `Mutable` or `Immutable`;
- its current generation;
- its next never-reused local `u16`, represented explicitly as available or
  exhausted;
- its current immutable snapshot locator.

An immutable snapshot records that table's ordered
`local encodedID <-> exact spelling` correspondence and integrity metadata.
The reverse spelling index is derived and scoped to that table only. Tables are
exact and case-sensitive: `"public"` and `"Public"` are different entries.
They perform no casing, normalization, derivation, or semantic interpretation.

Snapshot hashes are integrity and caching data. This model neither establishes
nor forecloses recursive content hashing of individual things.

Child-table ownership is structural. A table at an entry's full encodedID chain
is the table owned by that module; membership is not an attribute on a flat
global row.

## Allocation and sealing

Declarations allocate; references only resolve.

Within one atomic universe seal:

1. Validate the complete typed nested declaration graph.
2. Refuse the same exact spelling declared twice in one module table as a
   redefinition.
3. Process parent tables before their child tables.
4. Reuse the existing local encodedID for a spelling already present in its
   owning table.
5. Allocate unseen declaration spellings in canonical exact-byte order within
   each table.
6. Resolve references against committed state plus declarations staged by the
   seal. An unresolved reference allocates nothing.
7. Commit every affected head, immutable snapshot, cursor, and receipt
   atomically, or commit nothing.

Canonical ordering makes first allocation independent of declaration traversal
order and gives the same declaration set the same request digest. Allocation is
module-scoped: capacity exhaustion in one table never spills into another table,
silently widens the identifier, or reuses an old local encodedID.

Each successful seal receipt records the exact generation and immutable
snapshot locator for every table changed by that seal. Idempotent replay returns
those historical records unchanged even after later operations advance the
table heads.

The library provides generic state, staging, versioned archive, snapshot
integrity, and idempotent receipt mechanisms. The eventual translator daemon is
the sole persistent writer and owns authentication, authorization, durable
recovery, notifications, and its embedded sema database.

## Rename

Changing authored text is not an identity-preserving rename. If an Ethos text
edit changes an unseen spelling in a module table, the next seal allocates a
fresh encodedID and leaves the old entry allocated and orphaned. The allocation
site must describe this behavior in code. The seal contract has no continuation
field.

The operational rename is the sole identity-preserving path. It targets one full
encodedID chain and edits only the exact spelling stored for its final local ID
in the owning table. The chain, any child-table address, and every descendant
chain stay unchanged. Renaming a module is therefore the same operation as
renaming a member.

Rename reads the owning table head first. An immutable table returns a typed
immutable-table failure before any entry lookup. A mutable-table rename also
fails without writes for an unknown target, a spelling already present in that
table, stale state, conflicting idempotency content, corrupt state, or commit
failure.

There is no move, alias, delete, retire, freeze, thaw, or mutability-changing
operation in this target. Their policies are not to be inferred.

## Projection boundary

Human and language-specific names are views. Casing, composition,
disambiguation, and deterministic textual field names remain typed projection
data until a `TextualForm` evaluates them. Nomos may neither read nor construct
strings. The legacy eager walkers and interning of derived spellings must not be
treated as the target projection mechanism.

## Current code map

- `src/identity.rs` — `LocalEncodedId`, generic zero-or-more
  `TableAddress`, and non-empty `EncodedId`.
- `src/request.rs` — typed nested declarations, lookup-only references, and
  operational rename input.
- `src/state.rs` — table heads, immutable snapshots, cursors, typed integrity
  digests, and durable receipts.
- `src/table.rs` — module-scoped lookup, canonical seal application, rename,
  and stored-state validation.
- `src/transaction.rs` — pure staged seal and rename values with stale-base
  commit refusal and effect-free rollback.
- `src/archive.rs` — integrity-protected archive layout 3 and explicit
  rejection of incomplete earlier layouts.
- `src/name.rs` — exact spelling only; no eager casing or derivation.
- `src/error.rs` — typed no-write failures.
