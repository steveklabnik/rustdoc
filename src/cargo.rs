//! Functions for retrieving package data from `cargo`.

use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};

use indicatif::ProgressBar;
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

    /// The name of the target.
    ///
    /// This is *not* the name of the target's crate, which is used to retrieve the analysis data.
    /// Use the [`crate_name`] method instead.
    ///
    /// [`crate_name`]: ./struct.Target.html#method.crate_name
    pub name: String,
}

impl Target {
    /// Returns the name of the target's crate.
    ///
    /// This name is equivalent to the target's name, with dashes replaced by underscores.
    pub fn crate_name(&self) -> String {
        self.name.replace('-', "_")
    }
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
/// - `target`: The target that we should generate the analysis data for
pub fn generate_analysis(
    manifest_path: &Path,
    target: &Target,
    progress: &ProgressBar,
) -> Result<()> {
    let mut command = Command::new("cargo");

    command
        .arg("check")
        .arg("--manifest-path")
        .arg(manifest_path.join("Cargo.toml"))
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", manifest_path.join("target/rls"))
        .stderr(Stdio::piped())
        .stdout(Stdio::null());

    match target.kind {
        TargetKind::Library => {
            command.arg("--lib");
        }
        TargetKind::Binary => {
            command.args(&["--bin", &target.name]);
        }
    }

    let mut child = command.spawn()?;

    // Keep all stderr output in a buffer, in case we need to report it in the error.
    let mut stderr = String::new();

    // Display progress to the user.
    if let Some(ref mut out) = child.stderr {
        let out = BufReader::new(out);
        for line in out.lines() {
            let line = line?;
            stderr.push_str(&line);

            let line = line.trim();

            // Filter out lines that the user shouldn't see.
            //
            // `cargo check` will print any warnings and errors in the crate. Additionally,
            // `-Zsave-analysis` sometimes prints internal errors to stderr.
            //
            // We don't want to display any of these messages to the user, so we whitelist certain
            // cargo messages. Alternatively, we could use the JSON message format to filter, but
            // that is probably overkill.
            if line.starts_with("Updating") || line.starts_with("Compiling") ||
                line.starts_with("Finished")
            {
                progress.set_message(line);
            }
        }
    }

    let status = child.wait()?;

    if !status.success() {
        bail!(ErrorKind::Cargo(status, stderr));
    }

    Ok(())
}

/// Parse the library target from the crate metadata.
///
/// ## Arguments
///
/// - metadata: The JSON metadata of the crate.
pub fn target_from_metadata(metadata: &serde_json::Value) -> Result<Target> {
    // We can expect at least one package and target, otherwise the metadata generation would have
    // failed.
    let targets = metadata["packages"][0]["targets"].as_array().expect(
        "`targets` is not an array",
    );

    let mut targets = targets
        .into_iter()
        .flat_map(|target| {
            let name = target["name"].as_str().expect("`name` is not a string");
            let kinds = target["kind"].as_array().expect("`kind` is not an array");

            if kinds.len() != 1 {
                return Some(Err(
                    ErrorKind::Json(
                        format!("expected one kind for target '{}'", name),
                    ).into(),
                ));
            }

            let kind = match kinds[0].as_str().unwrap() {
                "lib" => TargetKind::Library,
                "bin" => TargetKind::Binary,
                _ => return None,
            };

            let target = Target {
                name: name.to_owned(),
                kind,
            };

            Some(Ok(target))
        })
        .collect::<Result<Vec<_>>>()?;

    if targets.is_empty() {
        bail!(ErrorKind::Json(
            "no targets with supported kinds (`bin`, `lib`) found"
                .into(),
        ));
    } else if targets.len() == 1 {
        Ok(targets.remove(0))
    } else {
        // FIXME(#105): Handle more than one target.
        print!("warning: Found more than one target to document. ");
        let (mut libs, mut bins): (Vec<_>, Vec<_>) =
            targets.into_iter().partition(|target| match target.kind {
                TargetKind::Library => true,
                TargetKind::Binary => false,
            });

        if !libs.is_empty() {
            println!("Documenting the library.");
            Ok(libs.remove(0))
        } else {
            let target = bins.remove(0);
            println!("Documenting the first binary: {}", target.name);
            Ok(target)
        }
    }
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
                            "kind": [ "lib" ],
                            "name": "underscored_name",
                        },
                    ],
                },
            ],
        });
        let target = super::target_from_metadata(&metadata).unwrap();
        assert_eq!(target, Target { kind: TargetKind::Library, name: "underscored_name".into() });
        assert_eq!(&target.crate_name(), "underscored_name");

        let metadata = json!({
            "packages": [
                {
                    "name": "dashed-name",
                    "targets": [
                        {
                            "kind": [ "lib" ],
                            "name": "dashed-name",
                        },
                    ],
                },
            ],
        });
        let target = super::target_from_metadata(&metadata).unwrap();
        assert_eq!(target, Target { kind: TargetKind::Library, name: "dashed-name".into() });
        assert_eq!(&target.crate_name(), "dashed_name");

        let metadata = json!({
            "packages": [
                {
                    "name": "underscored_name",
                    "targets": [
                        {
                            "kind": [ "bin" ],
                            "name": "underscored_name",
                        },
                    ],
                },
            ],
        });
        let target = super::target_from_metadata(&metadata).unwrap();
        assert_eq!(target, Target { kind: TargetKind::Binary, name: "underscored_name".into() });
        assert_eq!(&target.crate_name(), "underscored_name");

        let metadata = json!({
            "packages": [
                {
                    "name": "library",
                    "targets": [
                        {
                            "kind": [ "lib" ],
                            "name": "library",
                        },
                    ],
                },
            ],
        });
        assert_eq!(super::target_from_metadata(&metadata).unwrap().kind, TargetKind::Library);

        let metadata = json!({
            "packages": [
                {
                    "name": "binary",
                    "targets": [
                        {
                            "kind": [ "bin" ],
                            "name": "binary",
                        },
                    ],
                },
            ],
        });
        assert_eq!(super::target_from_metadata(&metadata).unwrap().kind, TargetKind::Binary);

        let metadata = json!({
            "packages": [
                {
                    "name": "library",
                    "targets": [
                        {
                            "kind": [ "lib" ],
                            "name": "library",
                        },
                        {
                            "kind": [ "test" ],
                            "name": "other_kind",
                        },
                    ],
                },
            ],
        });
        assert_eq!(super::target_from_metadata(&metadata).unwrap().kind, TargetKind::Library);
    }
}
