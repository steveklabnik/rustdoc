#![crate_type = "lib"]

//! Crate docs.
//!
//! Multiline docs

// @has data.type 'crate'
// @has data.id 'crates'

// @has data.attributes.docs 'Crate docs'
// @has data.attributes.summary 'Crate docs'
// @has data.attributes.docs 'Multiline docs'
// @!has data.attributes.summary 'Multiline docs'
