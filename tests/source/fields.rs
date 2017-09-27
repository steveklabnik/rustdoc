#![crate_type = "lib"]

// @has /data/relationships/structs/data/0/type 'struct'
// @has /data/relationships/structs/data/0/id 'fields::StructWithFields'

// @has /included/0/type 'struct'
// @has /included/0/id 'fields::StructWithFields'
// @has /included/0/attributes/name 'StructWithFields'
// @has /included/0/attributes/docs ''

// @has /included/0/relationships/child_fields/data/0/type 'field'

// @has /included/0/relationships/child_fields/data/0/id 'fields::StructWithFields::integer'

// @has /included/1/type 'field'
// @has /included/1/id 'fields::StructWithFields::integer'
// @has /included/1/attributes/name 'integer'
// @has /included/1/attributes/docs 'These are integer field docs.'

pub struct StructWithFields {
    /// These are integer field docs.
    pub integer: i32,
}
