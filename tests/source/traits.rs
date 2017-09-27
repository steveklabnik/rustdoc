#![crate_type = "lib"]

// @has /data/relationships/traits/data/0/type 'trait'
// @has /data/relationships/traits/data/0/id 'traits::EmptyTrait'

// @has /included/0/type 'trait'
// @has /included/0/id 'traits::EmptyTrait'
// @has /included/0/attributes/name 'EmptyTrait'
// @has /included/0/attributes/docs 'An empty trait.'

/// An empty trait.
pub trait EmptyTrait {}
