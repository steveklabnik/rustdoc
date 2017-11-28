#![crate_type = "lib"]

// Testing the child information:
//
// @has "included[?id=='nested_modules::example_module'] \
//          .relationships.modules.data[].type | [0]" \
//      "module"
// @has "included[?id=='nested_modules::example_module'] \
//          .relationships.modules.data[].id | [0]" \
//      "nested_modules::example_module::nested"
//
// Testing that we don't have a parent:
//
// @matches "included[?id=='nested_modules::example_module'] \
//          .relationships.parent" []
pub mod example_module {

    // For the child:
    //
    // @has "included[?id=='nested_modules::example_module::nested'] \
    //          .relationships.modules.data[].type | [0]" \
    //      "module"
    // @has "included[?id=='nested_modules::example_module::nested'] \
    //          .relationships.modules.data[].id | [0]" \
    //      "nested_modules::example_module::nested::nested2"
    //
    // For the parent:
    //
    // @has "included[?id=='nested_modules::example_module::nested'] \
    //          .relationships.parent.data.type | [0]" \
    //      "module"
    // @has "included[?id=='nested_modules::example_module::nested'] \
    //          .relationships.parent.data.id | [0]" \
    //      "nested_modules::example_module"
    pub mod nested {
        // For the parent:
        //
        // @has "included[?id=='nested_modules::example_module::nested::nested2'] \
        //          .relationships.parent.data.type | [0]" \
        //      "module"
        // @has "included[?id=='nested_modules::example_module::nested::nested2'] \
        //          .relationships.parent.data.id | [0]" \
        //      "nested_modules::example_module::nested"
        pub mod nested2 {}
    }
}
