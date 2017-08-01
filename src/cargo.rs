//! Functions for retrieving package data from `cargo`.

use std::path::Path;
use std::process::Command;

use serde_json;

use error::*;

/// Invoke cargo to generate the save-analysis data for the crate being documented.
///
/// ## Arguments
///
/// - manifest_path: The path containing the `Cargo.toml` of the crate
pub fn generate_analysis(manifest_path: &Path) -> Result<()> {
    // FIXME: Here we assume that we are documenting a library. This could be wrong, but it's the
    // common case, and it ensures that we are documenting the right target in the case that the
    // crate contains a binary and a library with the same name.
    //
    // Maybe we could use Cargo.toml's `doc = false` attribute to figure out the right target?
    let mut command = Command::new("cargo");
    command
        .arg("check")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest_path.join("Cargo.toml"))
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", manifest_path.join("target/rls"));

    let output = command.output()?;

    if !output.status.success() {
        return Err(
            ErrorKind::Cargo(
                output.status,
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ).into(),
        );
    }

    Ok(())
}

/// Grab the name of the binary or library from it's `Cargo.toml` file.
///
/// ## Arguments
///
/// - manifest_path: The path to the location of `Cargo.toml` of the crate being documented
pub fn crate_name_from_manifest_path(manifest_path: &Path) -> Result<String> {
    let mut command = Command::new("cargo");

    command
        .arg("metadata")
        .arg("--manifest-path")
        .arg(manifest_path.join("Cargo.toml"))
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1");

    let output = command.output()?;

    if !output.status.success() {
        return Err(
            ErrorKind::Cargo(
                output.status,
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ).into(),
        );
    }

    let metadata = serde_json::from_slice(&output.stdout)?;
    crate_name_from_metadata(&metadata)
}

/// Parse the crate name of the binary or library from crate metadata.
///
/// ## Arguments
///
/// - metadata: The JSON metadata of the crate.
fn crate_name_from_metadata(metadata: &serde_json::Value) -> Result<String> {
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
                    Some(name) => return Ok(name.replace('-', "_")),
                    None => return Err(ErrorKind::Json("target name is not a string").into()),
                }
            }
        }
    }

    Err(
        ErrorKind::Json("cargo metadata contained no targets").into(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn crate_name_from_metadata() {
        let metadata = json!({
            "packages": [
                {
                    "name": "underscored_name",
                    "targets": [
                        {
                            "crate_types": [ "lib" ],
                            "name": "underscored_name",
                        },
                    ],
                },
            ],
        });
        assert_eq!(&super::crate_name_from_metadata(&metadata).unwrap(), "underscored_name");

        let metadata = json!({
            "packages": [
                {
                    "name": "dashed-name",
                    "targets": [
                        {
                            "crate_types": [ "lib" ],
                            "name": "dashed-name",
                        },
                    ],
                },
            ],
        });
        assert_eq!(&super::crate_name_from_metadata(&metadata).unwrap(), "dashed_name");
    }
}
