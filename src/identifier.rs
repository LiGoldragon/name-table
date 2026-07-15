//! The stringless index every `Core*` type carries in place of a string.

use std::fmt;

/// The index every stringless `Core*` type holds where a name would otherwise
/// sit. It owns nothing but its representation — the [`NameTable`] is the noun
/// that carries meaning — so a `Core` value made only of `Identifier`s never
/// serializes a name, and a rename can never move its content identity.
///
/// [`NameTable`]: crate::NameTable
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct Identifier(u32);

impl Identifier {
    /// Wrap a raw index. Interning is the ordinary way to obtain an identifier;
    /// this exists for callers reconstructing one from a stored index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The raw index into the owning [`NameTable`].
    ///
    /// [`NameTable`]: crate::NameTable
    pub const fn value(self) -> u32 {
        self.0
    }

    /// The raw index as a `usize`, for slicing the table's name vector.
    pub const fn position(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "identifier {}", self.0)
    }
}
