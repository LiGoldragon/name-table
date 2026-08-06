# Name association architecture

`NameAssociationAuthority` is the only mutator and uses a private operating
system CSPRNG. `NameView` exposes reads; `NameChangeReplay` performs exact
reconstruction from an archiveable snapshot and realized changes.

The forward map keeps the current true name, revision, and live/tombstone
state for every issued encoded name. The reverse map is a true-name multimap:
identical bodies may intentionally have several distinct encoded names.
Textual visible/module/file metadata is keyed separately by encoded name.
