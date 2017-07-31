//! Functions for retrieving package data from `cargo`.

use serde_json;

use error::*;

/// Parse the crate name of the binary or library from crate metadata.
///
/// ## Arguments
///
/// - metadata: The JSON metadata of the crate.
pub fn crate_name_from_metadata(metadata: &serde_json::Value) -> Result<String> {
    let targets = match metadata["packages"][0]["targets"].as_array() {
        Some(targets) => targets,
        None => return Err(ErrorKind::Json("targets is not an array").into()),
    };

    for target in targets {
        let crate_types = match target["crate_types"].as_array() {
            Some(crate_types) => crate_types,
            None => return Err(ErrorKind::Json("crate types is not an array").into()),
        };

        for crate_type in crate_types {
            let ty = match crate_type.as_str() {
                Some(t) => t,
                None => {
                    return Err(
                        ErrorKind::Json("crate type contents are not a string").into(),
                    )
                }
            };

            if ty == "lib" {
                match target["name"].as_str() {
                    Some(name) => return Ok(name.to_string()),
                    None => return Err(ErrorKind::Json("target name is not a string").into()),
                }
            }
        }
    }

    Err(ErrorKind::Json("cargo metadata").into())
}
