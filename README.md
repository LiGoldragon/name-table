# name-table

`name-table` is the durable association authority for randomly issued opaque
`EncodedName` values. A `TrueName` is derived by the body owner from verified
canonical content; the table records one-to-many `{EncodedName TrueName}`
associations, content revisions, tombstones, and separate typed textual
metadata.

Every mutation appends its realized fact to the change log. Create stores the
random issued bytes, so replay reconstructs state without regenerating random
values or deriving content identities.
