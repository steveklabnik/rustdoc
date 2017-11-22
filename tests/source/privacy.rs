#![crate_type = "lib"]

// @has included[0].id 'privacy::public'
// @has included[0].attributes.docs 'Public module'
// @!has included[1].id 'privacy::public::private'
/// Public module.
pub mod public {
    mod private {}
}

// @!has included[1].id 'privacy::private'
// @!has included[1].attributes.docs 'Private module'
/// Private module.
mod private {}
