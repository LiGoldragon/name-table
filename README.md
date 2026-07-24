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
  serialization pre-image — a table's archive wire bytes are an explicitly
  versioned envelope containing its owned `NameSlice`: the namespace and ordered
  canonical names. The lookup index and borrowed slices are excluded from that
  archive.

## What it carries

- `Identifier` — a closed namespace variant with a namespace-local `u16` index;
  a `Core` value holds it in place of a string.
- `NameTable` — an interned, composable identifier space with exactly one owned
  home slice. `intern` allocates only in that home namespace; `compose` borrows
  another completed slice without copying names, flattening state, or renumbering
  identifiers. An owned slice archives independently and is composed again by its
  consumer after loading.
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

Version 0.1.0. This is crate L2 of the accepted language-family design. The
canonical micro-repository pins the published `content-identity` producer for
the shared `PortableArchive` and typed hash machinery. Consumers adopt published
micro-repository revisions producer-first.
