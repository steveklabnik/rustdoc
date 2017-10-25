#![crate_type = "lib"]

// @has data.relationships.types.data[0].type 'type'
// @has data.relationships.types.data[0].id 'types::ExampleType'

// @has included[0].type 'type'
// @has included[0].id 'types::ExampleType'
// @has included[0].attributes.name 'ExampleType'
// @has included[0].attributes.docs 'A type ExampleType.'

/// A type ExampleType.
pub type ExampleType = String;
