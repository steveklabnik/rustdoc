#![crate_type="lib"]

// @has /included/0/relationships/child_modules/data/0/type 'module'
// @has /included/0/relationships/child_modules/data/0/id 'nested_modules::example_module::nested'
mod example_module {

// @has /included/1/relationships/child_modules/data/0/type 'module'
// @has /included/1/relationships/child_modules/data/0/id 'nested_modules::example_module::nested::nested2'
    mod nested {
        mod nested2 {}
    }
}
