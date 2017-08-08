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

pub fn sample_function(x: i32) -> ExampleStruct {
    let _ = x;
    ExampleStruct
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

/// other nested 1
pub mod other_nested1 {
    /// other nested 2
    pub mod other_nested2 {
        /// nested 3
        pub mod other_nested3 {

        }
    }
}
