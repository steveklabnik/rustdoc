#![crate_type = "lib"]

// @has /data/relationships/child_modules/data/0/type 'module'
// @has /data/relationships/child_modules/data/0/id 'modules::module'

// @has /included/0/type 'module'
// @has /included/0/id 'modules::module'
// @has /included/0/attributes/name 'module'
// @has /included/0/attributes/docs 'a module'

/// a module
pub mod module {}
