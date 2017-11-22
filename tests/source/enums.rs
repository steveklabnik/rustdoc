#![crate_type = "lib"]

// @has data.relationships.enums.data[0].type 'enum'
// @has data.relationships.enums.data[0].id 'enums::SampleEnum'

// @has included[0].type 'enum'
// @has included[0].id 'enums::SampleEnum'
// @has included[0].attributes.name 'SampleEnum'
// @has included[0].attributes.docs 'An enum.'

// variants do not work yet, see https://github.com/nrc/rls-analysis/issues/98
// has included[0].relationships.enums.data[0].type 'enum'
// has included[0].relationships.enums.data[0].id 'enums::SampleEnum::EnumVariant'

/// An enum.
pub enum SampleEnum {
    /// An enum variant.
    EnumVariant,
}
