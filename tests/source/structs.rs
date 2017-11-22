#![crate_type = "lib"]

// @has "data.relationships.structs.data[?id=='structs::UnitStruct'].type | [0]" "struct"
// @has "included[?id=='structs::UnitStruct'].type | [0]" "struct"
// @has "included[?id=='structs::UnitStruct'].attributes.name | [0]" "UnitStruct"
// @has "included[?id=='structs::UnitStruct'].attributes.docs | [0]" "A unit struct."

/// A unit struct.
pub struct UnitStruct;

// @has "data.relationships.structs.data[?id=='structs::ContainerStruct'].type | [0]" "struct"
// @has "included[?id=='structs::ContainerStruct'].type | [0]" "struct"
// @has "included[?id=='structs::ContainerStruct'].attributes.name | [0]" "ContainerStruct"
// @has "included[?id=='structs::ContainerStruct'].attributes.docs | [0]" \
//      "A struct that contains another struct"

// @has "included[?id=='structs::ContainerStruct'].relationships.fields.data[] \
//         | [?id=='structs::ContainerStruct::inner_struct'].type | [0]" \
//      "field"

/// A struct that contains another struct.
/// Docs for the ContainerStruct.
pub struct ContainerStruct {
    /// These are docs for the inner struct.
    pub inner_struct: UnitStruct,
}
