#![crate_type = "lib"]

// @has  "included[?id=='summaries::Simple'] | [0].attributes.summary" 'Simple Summary'
// @has  "included[?id=='summaries::Simple'] | [0].attributes.plainSummary" 'Simple Summary'
// @!has "included[?id=='summaries::Simple'] | [0].attributes.summary" 'More detailed docs.'
// @!has "included[?id=='summaries::Simple'] | [0].attributes.plainSummary" 'More detailed docs.'

/// Simple Summary.
///
/// More detailed docs.
pub struct Simple;

// @has "included[?id=='summaries::Formatting'] | [0] \
//      .attributes.summary" 'Summary with \*formatting\*.'
// @has "included[?id=='summaries::Formatting'] | [0] \
//      .attributes.plainSummary" 'Summary with formatting.'

/// Summary with *formatting*.
///
/// More detailed docs.
pub struct Formatting;
