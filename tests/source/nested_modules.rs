#![crate_type = "lib"]

// @has /included/0/attributes/docs 'a module'

// @has /included/0/relationships/child_modules/data/0/type 'module'
// @has /included/0/relationships/child_modules/data/0/id 'mod2'

/// a module
pub mod mod1 {
    // @has /included/1/attributes/docs 'a second module'

    // @has /included/1/relationships/child_modules/data/0/type 'module'
    // @has /included/1/relationships/child_modules/data/0/id 'mod3'

    /// a second module
    pub mod mod2 {
        // @has /included/2/attributes/docs 'a third module'

        /// a third module
        pub mod mod3 {

        }
    }
}
