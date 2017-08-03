//! Functions for retrieving package data from `cargo`.

use std::path::Path;
use std::process::Command;

use serde_json;

use error::*;

/// The kinds of targets that we can document.
#[derive(Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// A `bin` target.
    Binary,

    /// A `lib` target.
    Library,
}

/// A target of documentation.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// The kind of the target.
    pub kind: TargetKind,

    /// The name of the target's crate.
    ///
    /// This name is equivalent to the target's name, with dashes replaced by underscores.
    pub crate_name: String,
}

/// Generate and parse the metadata of a cargo project.
///
/// ## Arguments
///
/// - `manifest_path`: The path containing the `Cargo.toml` of the crate
pub fn retrieve_metadata(manifest_path: &Path) -> Result<serde_json::Value> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(manifest_path.join("Cargo.toml"))
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .output()?;

    if !output.status.success() {
        return Err(
            ErrorKind::Cargo(
                output.status,
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ).into(),
        );
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Invoke cargo to generate the save-analysis data for the crate being documented.
///
/// ## Arguments
///
/// - `manifest_path`: The path containing the `Cargo.toml` of the crate
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

/// Parse the library target from the crate metadata.
///
/// ## Arguments
///
/// - metadata: The JSON metadata of the crate.
pub fn target_from_metadata(metadata: &serde_json::Value) -> Result<Target> {
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
                    Some(name) => {
                        let target = Target {
                            kind: TargetKind::Library,
                            crate_name: name.replace('-', "_"),
                        };
                        return Ok(target);
                    }
                    None => return Err(ErrorKind::Json("target name is not a string").into()),
                }
            }
        }
    }

    Err(
        ErrorKind::Json("cargo metadata contained no library targets").into(),
    )
}

#[cfg(test)]
mod tests {
    use super::{Target, TargetKind};

    #[test]
    fn target_from_metadata() {
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
        assert_eq!(
            super::target_from_metadata(&metadata).unwrap(),
            Target { kind: TargetKind::Library, crate_name: String::from("underscored_name"), }
        );

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
        assert_eq!(
            super::target_from_metadata(&metadata).unwrap(),
            Target { kind: TargetKind::Library, crate_name: String::from("dashed_name"), }
        );
    }
}
