# name-table

`name-table` defines pure association facts and capability contracts for
authority-issued opaque `EncodedName` values. A `TrueName` is derived by a
strict encoded body through the `TrueNamed` capability's `PortableArchive`
default. The facts describe associations, content revisions, tombstones, and
separate typed `TextualName` metadata.

Operational allocation, durable state, staging, commit, and replay belong to
the Sema authority, not this contract crate. Its create operation mints random
encoded bytes and records the realized fact, so replay never regenerates random
values or derives content identities.
