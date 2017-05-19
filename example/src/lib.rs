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

pub mod example_module {
    pub struct AnotherStruct;
}