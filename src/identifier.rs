//! The stringless identifier every `Core*` type carries in place of a name.

use std::fmt;

/// The closed registry of identifier namespaces.
///
/// A namespace is a component-owned allocation slice. Its position in this enum
/// is not data: an [`Identifier`] records the namespace in its variant, so two
/// equal locals from different namespaces are distinct by construction.
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
pub enum IdentifierNamespace {
    /// Names allocated by the schema component.
    Schema,
    /// Names allocated by the Logos component.
    Logos,
    /// Fixed, schema-independent Logos vocabulary.
    LogosStandard,
    /// Names allocated by the Nomos component.
    Nomos,
    /// Names used only by the structural-codec fixture universe.
    Fixture,
}

impl IdentifierNamespace {
    /// Construct this namespace's identifier for one local allocation.
    pub const fn identifier(self, local: u16) -> Identifier {
        match self {
            Self::Schema => Identifier::Schema(local),
            Self::Logos => Identifier::Logos(local),
            Self::LogosStandard => Identifier::LogosStandard(local),
            Self::Nomos => Identifier::Nomos(local),
            Self::Fixture => Identifier::Fixture(local),
        }
    }
}

/// The stringless identifier every `Core*` type carries where a name would
/// otherwise sit.
///
/// The enum variant is the namespace and the payload is that namespace's local
/// `u16` allocation. There is no flat integer representation or arithmetic
/// conversion between namespaces: `Schema(7)` and `Logos(7)` are different
/// values by type definition.
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
pub enum Identifier {
    Schema(u16),
    Logos(u16),
    LogosStandard(u16),
    Nomos(u16),
    Fixture(u16),
}

impl Identifier {
    /// This identifier's namespace variant.
    pub const fn namespace(self) -> IdentifierNamespace {
        match self {
            Self::Schema(_) => IdentifierNamespace::Schema,
            Self::Logos(_) => IdentifierNamespace::Logos,
            Self::LogosStandard(_) => IdentifierNamespace::LogosStandard,
            Self::Nomos(_) => IdentifierNamespace::Nomos,
            Self::Fixture(_) => IdentifierNamespace::Fixture,
        }
    }

    /// This identifier's namespace-local allocation.
    pub const fn local(self) -> u16 {
        match self {
            Self::Schema(local)
            | Self::Logos(local)
            | Self::LogosStandard(local)
            | Self::Nomos(local)
            | Self::Fixture(local) => local,
        }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(local) => write!(formatter, "Schema({local})"),
            Self::Logos(local) => write!(formatter, "Logos({local})"),
            Self::LogosStandard(local) => write!(formatter, "LogosStandard({local})"),
            Self::Nomos(local) => write!(formatter, "Nomos({local})"),
            Self::Fixture(local) => write!(formatter, "Fixture({local})"),
        }
    }
}
