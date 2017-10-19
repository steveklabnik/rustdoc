#![crate_type = "lib"]

// @has /data/relationships/statics/data/0/type 'static'
// @has /data/relationships/statics/data/0/id 'statics::EXAMPLE_STATIC'

// @has /included/0/type 'static'
// @has /included/0/id 'statics::EXAMPLE_STATIC'
// @has /included/0/attributes/docs 'A static.'
// @has /included/0/attributes/name 'EXAMPLE_STATIC'

/// A static.
pub static EXAMPLE_STATIC: i32 = 5;
