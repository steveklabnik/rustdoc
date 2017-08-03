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
        let mut command = Command::new("cargo");
        command
            .arg("check")
            .arg("--bins")
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
    println!("{:#?}", metadata);
    let mut root = match metadata["packages"][0]["name"].as_str() {
        Some(root) => root,
        None => return Err(ErrorKind::Json("resolve.root is not a string").into()),
    };

    match root.split_whitespace().nth(0) {
        Some(root) => {
            let kind = &metadata["packages"][0]["targets"][0]["crate_types"][0];
            println!("{:#?}", kind);
            let string = match kind.as_str() {
                Some(lib) => lib,
                None => return Err(ErrorKind::Json("kind is not a string").into()),
            };

            if string == "lib" {
                Ok(root.to_owned().replace('-', "_"))
            } else {
                Ok(root.to_owned())
            }
        },
        None => Err(ErrorKind::Json("No value for resolve.root was found").into()),
    }
}


/// Grab the name of the binary or library from it's `Cargo.toml` file.
///
/// ## Arguments
///
/// - manifest_path: The path to the location of `Cargo.toml` of the crate being documented
pub fn workspace_members_from_metadata(manifest_path: &Path) -> Result<Vec<String>> {
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

    let mut metadata = serde_json::from_slice(&output.stdout)?;
    workspace_paths_from_metadata(&mut metadata)
}

/// Parse the paths of the workspace members from crate metadata.
///
/// ## Arguments
///
/// - metadata: The JSON metadata of the crate.
fn workspace_paths_from_metadata(metadata: &mut serde_json::Value) -> Result<Vec<String>> {
    let workspace_members = match metadata["workspace_members"].as_array_mut() {
        Some(members) => members,
        None => return Err(ErrorKind::Json("workspace_members is not an array").into()),
    };

    // workspace_members values have the form of (where {} indicates values that change):
    // { wokspace_dir } { version } (path+file//{ path to dir})"
    // We want to extract the "path to dir" and return those values in an array as a manifest
    // path.
    let mut paths = Vec::new();

    for i in workspace_members.into_iter() {
        // We know this always matches
        let mut path = i.as_str().unwrap().to_owned().split("(path+file://").nth(1).unwrap().to_owned();
        // We don't care about the final ) value so take it out"
        let _ = path.pop();
        // Cargo.toml will be appended later so we need the '/' here
        path.push('/');
        paths.push(path);
    }

    Ok(paths)
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
