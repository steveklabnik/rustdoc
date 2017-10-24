#![crate_type = "lib"]

// @has /included/0/relationships/modules/data/0/type 'module'
// @has /included/0/relationships/modules/data/0/id 'nested_modules::example_module::nested'
pub mod example_module {

    // @has /included/1/relationships/modules/data/0/type 'module'
    // @has /included/1/relationships/modules/data/0/id \
    //      'nested_modules::example_module::nested::nested2'
    pub mod nested {
        pub mod nested2 {}
    }
}
