//! An example crate.
//!
//! This crate is used to test out the new rustdoc.
//!
//! It should have an example of almost everything, for testing
//! purposes.

/// An example struct.
///
/// This is mostly so that we can check the docs.
pub struct ExampleStruct;

impl ExampleStruct{
    /// An example method.
    ///
    /// Sup.
    pub fn example_method(&self) {
        println!("Hello world");
    }
}

/// An enum.
pub enum SampleEnum {
    /// An enum variant.
    EnumVariant,
}

/// An example function.
///
/// This function does stuff!
pub fn sample_function(x: i32) -> ExampleStruct {
    let _ = x;
    ExampleStruct
}

/// A type ExampleType.
type ExampleType = String;

/// An example macro.
macro_rules! example_macro {
    () => ( println!("Hello!"); )
}

/// An example union.
union example_union {
    f1: u32,
    f2: f32,
}

/// A const.
const EXAMPLE_CONST: i32 = 5;

/// A static.
static EXAMPLE_STATIC: i32 = 5;

/// A struct that contains another struct and other fields.
///
/// Docs for the ContainerStruct.
pub struct ContainerStruct {
    /// These are integer field docs.
    integer: i32,
    /// These are docs for the inner struct.
    inner_struct: ExampleStruct,
}

/// An example module
///
/// # Safety
///
/// Everything in this module is safe code, don't worry. If there was something unsafe,
/// we'd use **GLORIOUS BOLD** *and italics* to highlight it.
pub mod example_module {
    pub struct AnotherStruct;
}

/// nested 1
pub mod nested1 {
    /// nested 2
    pub mod nested2 {
        /// nested 3
        pub mod nested3 {

        }
    }
}
