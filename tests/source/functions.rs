#![crate_type = "lib"]

// @has data.relationships.functions.data[0].type 'function'
// @has data.relationships.functions.data[0].id 'functions::empty_function'

// @has included[0].type 'function'
// @has included[0].id 'functions::empty_function'
// @has included[0].attributes.name 'empty_function'
// @has included[0].attributes.docs 'An empty function.'

/// An empty function.
pub fn empty_function() {}
