#![crate_type = "lib"]

/// A unit struct.
pub struct UnitStruct;

// @has /included/0/type 'struct'
// @has /included/0/id 'structs::UnitStruct'
// @has /included/0/attributes/name 'UnitStruct'
// @has /included/0/attributes/docs 'A unit struct.'
// @has /data/relationships/child_structs/data/0/id 'structs::UnitStruct'
