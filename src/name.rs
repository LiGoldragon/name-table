//! The interned name and the one home of the derived-name rule.
//!
//! Capitalization is semantic in this family: a capitalized-leading name is an
//! object, a lowercase-leading name is a name. The derived-name rule — a field
//! name is the `snake_case` of its type name — lives HERE as methods on [`Name`],
//! consolidating the two walkers that were hand-written independently across the
//! fleet:
//!
//! - `schema`'s `Name::field_name` (`schema/src/schema.rs:50-65`, 16 call sites):
//!   PascalCase to `snake_case`.
//! - `schema-rust`'s `ScreamingName::screaming` (`schema-rust/src/lib.rs:2178-2204`):
//!   PascalCase to `SCREAMING_SNAKE_CASE`.
//!
//! Both are the same word-boundary walk under two casings; [`DerivedCasing`] names
//! that difference as data so the loop lives once. `schema`'s walker first strips
//! a namespace prefix through its own `local_part()`; that namespace split is a
//! schema concern and is excluded here — on a bare (non-namespaced) name the two
//! behaviors are identical, which is what the ported tests assert.

/// A name, interned into a [`NameTable`]. In the stringless substrate this is the
/// only place a name's text lives; every `Core*` value holds an [`Identifier`]
/// into the table instead.
///
/// [`NameTable`]: crate::NameTable
/// [`Identifier`]: crate::Identifier
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Name(String);

/// Which casing a derived-name walk emits at a PascalCase word boundary. This is
/// the single axis on which `schema`'s `field_name` and `schema-rust`'s
/// `screaming` differ, named as data so the boundary walk is written once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DerivedCasing {
    /// `snake_case`: lowercase every letter, underscore before an interior
    /// word boundary. (`schema`'s `field_name`.)
    Snake,
    /// `SCREAMING_SNAKE_CASE`: uppercase every letter, underscore before an
    /// interior word boundary. (`schema-rust`'s `screaming`.)
    ScreamingSnake,
}

impl DerivedCasing {
    /// Render a boundary letter (an ASCII uppercase in the source name) under
    /// this casing, keeping its own case for screaming and lowering it for snake.
    fn render_boundary_letter(self, letter: char) -> char {
        match self {
            Self::Snake => letter.to_ascii_lowercase(),
            Self::ScreamingSnake => letter,
        }
    }

    /// Render an ordinary (non-boundary, non-separator) character under this
    /// casing.
    fn render_ordinary(self, character: char) -> char {
        match self {
            Self::Snake => character,
            Self::ScreamingSnake => character.to_ascii_uppercase(),
        }
    }
}

impl Name {
    /// Intern-facing constructor from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The name's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The single derived-name boundary walk, parameterized by casing. An ASCII
    /// uppercase letter opens a word (underscore before it unless it leads); a
    /// `-` becomes `_`; every other character is rendered under the casing.
    fn derived_name(&self, casing: DerivedCasing) -> String {
        let mut output = String::new();
        for (index, character) in self.0.chars().enumerate() {
            if character.is_ascii_uppercase() {
                if index > 0 {
                    output.push('_');
                }
                output.push(casing.render_boundary_letter(character));
            } else if character == '-' {
                output.push('_');
            } else {
                output.push(casing.render_ordinary(character));
            }
        }
        output
    }

    /// The `snake_case` field name derived from this type name. The consolidated
    /// home of `schema`'s `Name::field_name` walker.
    pub fn field_name(&self) -> String {
        self.derived_name(DerivedCasing::Snake)
    }

    /// The `SCREAMING_SNAKE_CASE` constant name derived from this type name. The
    /// consolidated home of `schema-rust`'s `ScreamingName::screaming` walker.
    pub fn screaming(&self) -> String {
        self.derived_name(DerivedCasing::ScreamingSnake)
    }

    /// The `PascalCase` object spelling, reconstructed from a `snake_case` or
    /// `kebab-case` name. This is the inverse of [`field_name`](Self::field_name):
    /// each `_`/`-`-separated segment is capitalized and concatenated, so
    /// `Name::new("commit_sequence").pascal_case() == "CommitSequence"`. It is the
    /// third derived form the two source walkers jointly imply — a round-trip
    /// partner rather than a lift — kept here so all derived-name spelling has one
    /// home.
    pub fn pascal_case(&self) -> String {
        let mut output = String::new();
        let mut at_segment_start = true;
        for character in self.0.chars() {
            if character == '_' || character == '-' {
                at_segment_start = true;
            } else if at_segment_start {
                output.push(character.to_ascii_uppercase());
                at_segment_start = false;
            } else {
                output.push(character);
            }
        }
        output
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value)
    }
}
