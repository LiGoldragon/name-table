# name-table architecture

## Overview

`name-table` is crate L2 of the shared-codec language family. The family's rule
is that dependencies run strictly downward and stringless Core never depends on
text: `content-identity <- name-table <- raw-discovery <- structural-codec`. This
crate holds the identifier space every `Core*` type indexes into, and depends
only on `content-identity` (for the shared `PortableArchive` rkyv discipline) and
`rkyv`.

## Direction

The psyche's settled rulings this crate embodies:

- All `Core*` types are stringless; every identifier is an index into the
  corresponding `NameTable`, and all names live here.
- One continuous identifier space extends schema's allocation into logos
  (`extend_from`): a carried-over identifier keeps its exact index.
- The `NameTable` is excluded from `Core` content hashes by construction —
  renaming is a `NameTable`-only edit and never moves `Core` identity. Names and
  `Core` values never serialize together.
- Capitalization is semantic: capitalized-leading is an object, lowercase-leading
  is a name; the derived-name rule (field name = `snake_case` of type name) lives
  here as methods on `Name` — one home for the walkers duplicated in `schema` and
  `schema-rust`.
- Interning is transactional: a failed decode alternative leaves no allocation
  effect. A real staging surface provides this, not a convention.
- Named views (`Textual*`) are derived from `Core` + `NameTable`, never stored.
- A `NameTable` may later be stored as a first-class co-versioned sibling of
  `Core` in daemon stores, so it must be cleanly archivable; where it is stored is
  not this crate's business.

Consumption and integration will readapt to the forthcoming release-train flow.
This crate migrates no consumer; schema, nomos, logos, and the rest adopt these
types in later train slices.

## Components and boundaries

- `Identifier` (`src/identifier.rs`) — a `u32` newtype, rkyv-archivable, the index
  a `Core` value carries in place of a string.
- `Name` (`src/name.rs`) — the interned name and the one home of the derived-name
  rule. The two source walkers (`schema`'s `field_name`, `schema-rust`'s
  `screaming`) are the same word-boundary walk under two casings; `DerivedCasing`
  names that difference as data so the loop is written once. `pascal_case` is the
  inverse round-trip partner.
- `NameTable` (`src/table.rs`) — the interned, append-only, index-stable
  identifier space. Its canonical archivable state is the ordered name vector
  alone; the name-to-identifier lookup is a derived accelerator, rebuilt on load
  and never serialized. `NameTableDomain` gives the table its own content
  identity for co-versioned sibling storage.
- `NameTransaction` (`src/transaction.rs`) — the speculative interning overlay.
- `NameResolver` / `NameInterner` (`src/boundary.rs`) — the two codec-boundary
  capabilities, threaded down a codec call tree, never held by a node.
- `TextualProjection` (`src/projection.rs`) — the derive-a-named-view surface.
- `NameTableError` (`src/error.rs`) — the typed crate-boundary error (thiserror).

## Names never serialize with Core values

This is structural, not a runtime check. A `Core` value is built from
`Identifier` indices and holds no names, so no name can enter its content-hash
pre-image (`content-identity` hashes the stringless bytes). A `NameTable`
serializes only its ordered names (`to_archive_bytes` over the name vector; the
lookup index is derived, never archived). The two data shapes have disjoint
pre-images, so a rename — a table-only edit — cannot move any `Core` address. The
`archive` and `transaction` test suites prove the byte-level and identity-level
stability.

## The transactional contract

The accepted hardening requires that a failed decode alternative leave no
allocation effect. `name-table` meets it structurally rather than by undo: a
`NameTransaction` stages new names on the side and never mutates the committed
table until `commit`. A dropped or rolled-back transaction therefore leaves the
table byte-identical by construction — there is nothing to undo. A decode runs
each alternative inside a transaction (`begin`, or the `try_intern` closure form)
and commits only the winner; the loser is dropped and leaks nothing.

## The one home for the walkers

The derived-name rule was hand-written independently as `schema`'s
`Name::field_name` (PascalCase to `snake_case`, 16 call sites) and `schema-rust`'s
`ScreamingName::screaming` (PascalCase to `SCREAMING_SNAKE`). Both are the same
word-boundary walk. Here they are one private walk (`Name::derived_name`)
parameterized by a `DerivedCasing`, with `field_name` and `screaming` as the two
public spellings and `pascal_case` as the inverse. schema's walker first strips a
namespace through its own `local_part()`; that namespace split is a schema concern
and is deliberately excluded here — on a bare name the behaviors are identical,
which the `walkers` tests assert against the exact ported expectations.

## Constraints

- Every function is a method on a data-bearing type or a trait impl; no free
  helpers outside test code.
- Domain values are typed newtypes; the one casing axis is a `DerivedCasing` enum,
  not a boolean.
- Typed errors at the boundary; no `anyhow`/`eyre`.
- No unsafe code (`unsafe_code = "forbid"`).
- A `NameTable`'s archived bytes are its names and nothing else; the lookup index
  is never serialized.

## Invariants

- Interning is deterministic: a name interns to the same identifier every time
  within one table lineage.
- Identifiers are index-stable: an identifier's index never changes once
  allocated, so `extend_from` is a continuous space.
- A rolled-back or dropped transaction leaves the table byte-identical, down to
  `to_archive_bytes` and `identity` (proven in `tests/transaction.rs`).
- `field_name`/`screaming` reproduce the two source walkers exactly (proven in
  `tests/walkers.rs`).

## Code map

- `src/lib.rs` — module root and public re-exports.
- `src/identifier.rs` — `Identifier`.
- `src/name.rs` — `Name`, `DerivedCasing`, the derived-name walkers.
- `src/table.rs` — `NameTable`, `NameTableDomain`.
- `src/transaction.rs` — `NameTransaction`.
- `src/boundary.rs` — `NameResolver`, `NameInterner`.
- `src/projection.rs` — `TextualProjection`.
- `src/error.rs` — `NameTableError`.
- `tests/interning.rs` — determinism, resolve round-trips, boundary capabilities.
- `tests/continuity.rs` — `extend_from` index stability.
- `tests/transaction.rs` — rollback/commit and the interning-atomicity law.
- `tests/walkers.rs` — derived-name outputs vs the ported source expectations.
- `tests/archive.rs` — portable archive round-trip of a populated table.
- `tests/projection.rs` — the Textual-projection surface and rename-stability.
