#![crate_type = "lib"]

// @has data.relationships.modules.data[0].[type,id] '["module", "modules::module"]'

// @has 'included[0].[type, id]' '["module", "modules::module"]'
// @has 'included[0].attributes.name' "module"
// @has 'included[0].attributes.docs | contains(@, `"a module"`)'

/// a module
pub mod module {}
