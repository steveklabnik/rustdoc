#![crate_type = "lib"]

// @has /data/relationships/structs/data/0/type 'struct'
// @has /data/relationships/structs/data/0/id 'structs::UnitStruct'

// @has /included/0/type 'struct'
// @has /included/0/id 'structs::UnitStruct'
// @has /included/0/attributes/name 'UnitStruct'
// @has /included/0/attributes/docs 'A unit struct.'

/// A unit struct.
pub struct UnitStruct;

// @has /data/relationships/structs/data/1/type 'struct'
// @has /data/relationships/structs/data/1/id 'structs::ContainerStruct'

// @has /included/1/type 'struct'
// @has /included/1/id 'structs::ContainerStruct'
// @has /included/1/attributes/name 'ContainerStruct'
// @has /included/1/attributes/docs 'A struct that contains another struct.'

// @has /included/1/relationships/fields/data/0/type 'field'
// @has /included/1/relationships/fields/data/0/id 'structs::ContainerStruct::inner_struct'

/// A struct that contains another struct.
/// Docs for the ContainerStruct.
pub struct ContainerStruct {
    /// These are docs for the inner struct.
    pub inner_struct: UnitStruct,
}
