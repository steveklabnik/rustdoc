//! Function and data type declarations for dealing with Rust's different types of items available
//! to it like a `struct`, `const`, or an `enum`

use analysis::raw::DefKind;

/// Struct containing different types of data about definitions possibly in a Rust Code Base
#[derive(Debug)]
pub struct Metadata {
    /// What kind of definition: module, struct, function, etc.
    pub kind: DefKind,

    /// Full path to the item in the module hierarchy
    pub qualified_name: String,

    /// Name of the definition
    pub name: String,

    /// Documentation associated with the definition
    pub docs: String,
}
