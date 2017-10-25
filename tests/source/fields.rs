#![crate_type = "lib"]

// @has "data.relationships.structs.data[?id=='fields::StructWithFields'].type | [0]" "struct"

// @has "included[?id=='fields::StructWithFields'].type | [0]" "struct"
// @has "included[?id=='fields::StructWithFields'].attributes.name | [0]" "StructWithFields"
// @has "included[?id=='fields::StructWithFields'].attributes.docs| [0]" ""
// @has "included[?id=='fields::StructWithFields'].relationships.fields.data[] \
//         | [?id=='fields::StructWithFields::integer'].type | [0]"  \
//      "field"

// @has "included[?id=='fields::StructWithFields::integer'].type | [0]" "field"
// @has "included[?id=='fields::StructWithFields::integer'].attributes.name | [0]" "integer"
// @has "included[?id=='fields::StructWithFields::integer'].attributes.docs | [0]" \
//      "These are integer field docs."

pub struct StructWithFields {
    /// These are integer field docs.
    pub integer: i32,
}
