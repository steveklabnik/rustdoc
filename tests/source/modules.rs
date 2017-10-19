#![crate_type = "lib"]

// @has /data/relationships/modules/data/0/type 'module'
// @has /data/relationships/modules/data/0/id 'modules::module'

// @has /included/0/type 'module'
// @has /included/0/id 'modules::module'
// @has /included/0/attributes/name 'module'
// @has /included/0/attributes/docs 'A module.'

/// A module.
pub mod module {}
