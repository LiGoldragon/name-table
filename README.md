# name-table

The stringless-Core identifier space: `Identifier` indices, the interning
`NameTable` with a transactional staging surface, and the one home for the
derived-name walkers.

This is crate L2 of the shared-codec language family. The family's rule is that
dependencies run strictly downward and stringless Core never depends on text:
`content-identity <- name-table <- raw-discovery <- structural-codec`. This crate
sits just above the leaf and depends only on `content-identity` (for the shared
`PortableArchive` rkyv discipline) and `rkyv`.

## The stringless principle

Every `Core*` type in the family is stringless: it carries `Identifier` indices,
never names. All names live here, in a `NameTable`. A `Core` value made only of
identifiers has no name in its bytes, so:

- **Rename-stability by construction.** Content identity (from `content-identity`,
  over a `Core` value's stringless bytes) never folds a name. A rename is a
  `NameTable`-only edit that changes how identifiers resolve, and can never move
  `Core` identity. Names and `Core` values are structurally incapable of sharing a
  serialization pre-image — a table's canonical bytes are its ordered names and
  nothing else.

## What it carries

- `Identifier` — the `u32` index a `Core` value holds in place of a string.
- `NameTable` — an interned, append-only, index-stable identifier space:
  `intern`, `resolve`, and `extend_from`. `extend_from` is the one continuous
  identifier space that carries schema's allocation into logos: the extension
  begins with every base name at its exact identifier, and new names append above.
- `NameTransaction` — a speculative interning overlay that merges on commit. A
  failed decode alternative leaves no allocation effect, because the committed
  table is never mutated until commit — a dropped transaction is an effect-free
  rollback by construction, not by undo.
- `Name` — the interned name and the one home of the derived-name rule:
  `field_name` (PascalCase to `snake_case`), `screaming` (to
  `SCREAMING_SNAKE_CASE`), `pascal_case` (the inverse). These consolidate walkers
  that were hand-written independently in `schema` (`Name::field_name`) and
  `schema-rust` (`ScreamingName::screaming`).
- `NameResolver` / `NameInterner` — the two codec-boundary capabilities: the
  read-only view an encode path is threaded, and the mutating view a decode path
  is threaded. A codec never holds the whole table, only the capability its
  direction needs.
- `TextualProjection` — the surface for deriving a named `Textual*` view from a
  `Core` value plus a table. The named view is derived on demand, never stored;
  concrete `Textual*` types belong to later crates.

## The transactional contract

Interning is transactional so a failed decode never leaks a name. Run each decode
alternative inside a transaction (`NameTable::begin`, or `NameTable::try_intern`
for the closure form) and commit only the winner:

- Names interned through the transaction stage on the side, at the identifiers
  they would occupy after a commit.
- `commit` merges the staged names into the committed table.
- `rollback` (or simply dropping the transaction) discards the staging buffer; the
  committed table is untouched, down to its archived bytes and its identity.

## Build and test

```sh
nix flake check      # build, test, clippy, fmt, doc — the gate
cargo test           # inner-loop tests
```

## Status

Version 0.1.0. This is slice one, crate L2 of the accepted language-family
design. It depends on `content-identity` (crate L1) pinned by git revision for
the shared `PortableArchive` bound. Consumption and integration — schema, nomos,
logos, and the other consumers adopting these types — will readapt to the
forthcoming release-train flow.
