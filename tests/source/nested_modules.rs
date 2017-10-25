#![crate_type = "lib"]

// @has "included[?id=='nested_modules::example_module'] \
//          .relationships.modules.data[].type | [0]" \
//      "module"
// @has "included[?id=='nested_modules::example_module'] \
//          .relationships.modules.data[].id | [0]" \
//      "nested_modules::example_module::nested"
pub mod example_module {

    // @has "included[?id=='nested_modules::example_module::nested'] \
    //          .relationships.modules.data[].type | [0]" \
    //      "module"
    // @has "included[?id=='nested_modules::example_module::nested'] \
    //          .relationships.modules.data[].id | [0]" \
    //      "nested_modules::example_module::nested::nested2"
    pub mod nested {
        pub mod nested2 {}
    }
}
