#![crate_type = "lib"]

// @has data.relationships.consts.data[0].type 'const'
// @has data.relationships.consts.data[0].id 'consts::EXAMPLE_CONST'

// @has included[0].type 'const'
// @has included[0].id 'consts::EXAMPLE_CONST'
// @has included[0].attributes.docs 'A const.'
// @has included[0].attributes.name 'EXAMPLE_CONST'

/// A const.
pub const EXAMPLE_CONST: i32 = 5;
