# Name association architecture

`NameAssociationAuthority`, `NameView`, and `NameChangeReplay` are capability
contracts. The Sema authority is their concrete mutator, CSPRNG owner, and
replay implementation; this crate intentionally owns none of that state.

The Sema authority's forward association state keeps the current true name,
revision, and live/tombstone state for every issued encoded name. The contract
does not expose a true-name-to-encoded-name reverse lookup. Textual
name/module/file metadata is keyed separately by encoded name.
