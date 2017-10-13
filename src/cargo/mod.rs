//! Functions for retrieving package data from `cargo`.

use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;

use serde_json;

use Config;
use Verbosity;
use error::*;
use ui::Ui;

mod commands;
mod command_bridge;
mod target;
pub use cargo::target::*;

/// Generate and parse the metadata of a cargo project.
///
/// ## Arguments
///
/// - `manifest_path`: The path to the crate's `Cargo.toml`
pub fn retrieve_metadata(manifest_path: &Path) -> Result<serde_json::Value> {
    let output = commands::retrieve_metadata(manifest_path)
        .to_command()
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
/// - `config`: Rustdoc configuration
/// - `target`: The target that we should generate the analysis data for
/// - `report_progress`: A closure that should be called to report a progress message
pub fn generate_analysis<F>(config: &Config, _target: &Target, report_progress: F) -> Result<()>
where
    F: Fn(&str) -> (),
{
    let is_verbose = &Verbosity::Verbose == config.ui.verbosity();
    let mut command = commands::generate_analysis(&config.manifest_path, is_verbose)?
        .to_command();

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
                line.starts_with("Finished") || line.starts_with("Running") ||
                line.starts_with("Fresh") || line.starts_with("Downloading")
            {
                report_progress(line);
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
pub fn target_from_metadata(ui: &Ui, metadata: &serde_json::Value) -> Result<Target> {
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
        let (mut libs, mut bins): (Vec<_>, Vec<_>) =
            targets.into_iter().partition(|target| match target.kind {
                TargetKind::Library => true,
                TargetKind::Binary => false,
            });

        // Default to documenting the library if it exists.
        let target = if !libs.is_empty() {
            libs.remove(0)
        } else {
            bins.remove(0)
        };

        let kind = match target.kind {
            TargetKind::Library => "library",
            TargetKind::Binary => "first binary",
        };

        ui.warn(&format!(
            "Found more than one target to document. Documenting the {}: {}",
            kind,
            target.name
        ));

        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use ui::Ui;
    use super::{Target, TargetKind};

    #[test]
    fn target_from_metadata() {
        let ui = Ui::default();

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
        let target = super::target_from_metadata(&ui, &metadata).unwrap();
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
        let target = super::target_from_metadata(&ui, &metadata).unwrap();
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
        let target = super::target_from_metadata(&ui, &metadata).unwrap();
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
        assert_eq!(super::target_from_metadata(&ui, &metadata).unwrap().kind, TargetKind::Library);

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
        assert_eq!(super::target_from_metadata(&ui, &metadata).unwrap().kind, TargetKind::Binary);

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
        assert_eq!(super::target_from_metadata(&ui, &metadata).unwrap().kind, TargetKind::Library);
    }
}
