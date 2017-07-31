//! Function and data type declarations for dealing with Rust's different types of items available
//! to it like a `struct`, `const`, or an `enum`

#[derive(Debug)]
/// Enum containing different types of data about an items possibly in a Rust Code Base
pub enum Metadata {
    /// Data associated with module definitions
    Module {
        /// Full path to the item in the module hierarchy
        qualified_name: String,
        /// Name of the module
        name: String,
        /// Doc String associated with the module
        docs: String,
    },
    /// Data associated with `static` definitions
    Static {},
    /// Data associated with `const` definitions
    Const {},
    /// Data associated with `enum` definitions
    Enum {},
    /// Data associated with `struct` definitions
    Struct {},
    /// Data associated with `union` definitions
    Union {},
    /// Data associated with `trait` definitions
    Trait {},
    /// Data associated with `fn` definitions
    Function {},
    /// Data associated with `macro` definitions
    Macro {},
    /// Data associated with `tuple` definitions
    Tuple {},
    /// Data associated with `method` definitions
    Method {},
    /// Data associated with `type` definitions
    Type {},
    /// Data associated with `local` definitions
    Local {},
    /// Data associated with `field` definitions
    Field {},
}
