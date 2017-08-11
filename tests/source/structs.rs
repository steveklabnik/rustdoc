#![crate_type = "lib"]

/// A unit struct.
pub struct UnitStruct;

// @has 'included[0].[type, id]' '["struct", "structs::UnitStruct"]'
// @has 'included[0].attributes.name' "UnitStruct"
// @has 'included[0].attributes.docs | contains(@, `"A unit struct."`)'
// @has 'data.relationships.structs.data[0].id' "structs::UnitStruct"
