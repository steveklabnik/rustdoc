//! Functions used to generate the documentation for Rust Crates.

#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

#[macro_use]
extern crate failure;
#[cfg_attr(test, macro_use)]
#[cfg(test)]
extern crate indoc;
#[macro_use]
extern crate log;
#[cfg_attr(test, macro_use)]
extern crate quote;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;

extern crate indicatif;
extern crate open;
extern crate pulldown_cmark;
extern crate rls_analysis as analysis;
extern crate rls_data as analysis_data;
extern crate syn;
extern crate tempdir;

pub mod cargo;
pub mod error;
pub mod json;

mod test;
mod ui;

use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cargo::Target;
use json::Documentation;
use ui::Ui;

pub use json::create_documentation;
pub use ui::Verbosity;

use failure::Error;
use failure::Fail;
/// The general Result type for `rustdoc`.
///
/// Similar to `std::io::Result` and friends, this makes error handling a bit more succinct.
pub type Result<T> = std::result::Result<T, Error>;

/// A structure that contains various fields that hold data in order to generate doc output.
#[derive(Debug)]
pub struct Config {
    /// Interactions with the user interface.
    ui: Ui,

    /// Path to the `Cargo.toml` file for the crate being analyzed
    manifest_path: PathBuf,

    /// Path to place rustdoc output
    output_path: Option<PathBuf>,

    /// Contains the Cargo analysis output for the crate being documented
    host: analysis::AnalysisHost,
}

impl Config {
    /// Create a new `Config` based off the location of the manifest as well as assets generated
    /// during the build phase
    ///
    /// ## Arguments
    ///
    /// - `manifest_path`: The path to the `Cargo.toml` of the crate being documented
    pub fn new(verbosity: Verbosity, manifest_path: PathBuf) -> Result<Config> {
        let host = analysis::AnalysisHost::new(analysis::Target::Debug);

        if !manifest_path.is_file() || !manifest_path.ends_with("Cargo.toml") {
            return Err(failure::err_msg(
                "The --manifest-path must be a path to a Cargo.toml file",
            ));
        }

        Ok(Config {
            ui: Ui::new(verbosity),
            manifest_path,
            output_path: None,
            host,
        })
    }

    /// Returns the directory containing the `Cargo.toml` of the crate being documented.
    pub fn root_path(&self) -> &Path {
        // unwrap() is safe, as manifest_path will point to a file
        self.manifest_path.parent().unwrap()
    }

    /// Returns the directory where output files should be placed
    pub fn output_path(&self) -> PathBuf {
        match self.output_path {
            Some(ref path) => path.clone(),
            None => self.root_path().join("target").join("doc"),
        }
    }

    /// Set the directory where output files should be placed
    pub fn set_output_path(&mut self, output_path: PathBuf) {
        self.output_path = Some(output_path);
    }

    /// Returns the path to the generated documentation.
    pub fn documentation_path(&self) -> PathBuf {
        self.output_path().join("data.json")
    }

    /// Open the generated docs in a web browser.
    pub fn open_docs(&self) -> Result<()> {
        let mut index = self.output_path().join("index.html");

        // If we can't find the index at the root, try looking in the crate folder.
        if !index.is_file() {
            let metadata = cargo::retrieve_metadata(&self.manifest_path)?;
            let target = cargo::target_from_metadata(&self.ui, &metadata)?;
            index = self.output_path()
                .join(target.crate_name())
                .join("index.html");
        }

        open::that(index)?;
        Ok(())
    }
}

/// Generate documentation for a crate. This can be tuned to output JSON and/or Web assets to view
/// documentation or use the JSON for other applications built on top of `rustdoc`.
///
/// ## Arguments
///
/// - `config`: The `Config` struct that contains the data needed to generate the documentation
/// - `artifacts`: A slice containing what assets should be output at the end
pub fn build(config: &Config, artifacts: &[&str]) -> Result<()> {
    let metadata = cargo::retrieve_metadata(&config.manifest_path)?;
    let target = cargo::target_from_metadata(&config.ui, &metadata)?;
    generate_and_load_analysis(config, &target)?;

    let output_path = config.output_path();
    fs::create_dir_all(&output_path)?;

    let json = {
        let task = config.ui.start_task("Generating JSON");
        task.report("In Progress");
        let docs = json::create_documentation(&config.host, &target.crate_name())?;
        serde_json::to_string(&docs)?
    };

    if artifacts.contains(&"json") {
        let json_path = output_path.join("data.json");
        let mut file = File::create(json_path)?;
        file.write_all(json.as_bytes())?;
    }

    // Now that we've generated the documentation JSON, we start the frontend as a subprocess to
    // generate the final output.
    if artifacts.contains(&"frontend") {
        let task = config.ui.start_task("Generating documentation");
        task.report("In Progress");

        let frontend = env::var("RUSTDOC_FRONTEND").unwrap_or_else(|_| {
            // If we're being run from cargo, use the binary in this repository. Otherwise, try to
            // find one in PATH.
            if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
                let profile = if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                };

                format!("{}/target/{}/rustdoc-ember", manifest_dir, profile)
            } else {
                String::from("rustdoc-ember")
            }
        });

        let mut frontend_proc = Command::new(&frontend)
            .arg("--output")
            .arg(config.output_path())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    e.context(format!("frontend `{}` not found", frontend))
                        .into()
                } else {
                    failure::Error::from(e)
                }
            })?;

        {
            let stdin = frontend_proc.stdin.as_mut().unwrap();
            stdin.write_all(json.as_bytes())?;
        }

        let output = frontend_proc.wait_with_output()?;
        if !output.status.success() {
            task.error();
            drop(task);
            println!("\n{}", String::from_utf8_lossy(&output.stderr));
            return Err(format_err!(
                "frontend `{}` did not execute successfully",
                frontend
            ));
        }
    }

    Ok(())
}

/// Run all documentation tests.
pub fn test(config: &Config) -> Result<()> {
    let doc_json = File::open(config.documentation_path())
        .map_err(|e| failure::Error::from(e.context("could not find generated documentation")))?;
    let docs: Documentation = serde_json::from_reader(doc_json)?;

    // TODO a better way to find crate name?
    let krate = docs.data.as_ref().unwrap();
    let crate_name = krate.id.split("::").next().unwrap();

    let location = config.output_path().join("tests");
    let tests = {
        let task = config.ui.start_task("Finding tests");
        task.report("In Progress");
        test::find_tests(&docs)
    };

    {
        let task = config.ui.start_task("Saving tests");
        task.report("In Progress");
        test::save_tests(&tests, &location, &crate_name)?;
    }

    let binary = {
        let task = config.ui.start_task("Compiling tests");
        task.report("In Progress");
        test::compile_tests(&config, &location)?
    };

    {
        let task = config.ui.start_task("Executing tests");
        task.report("In Progress");
        test::execute_tests(&binary)?;
    }

    Ok(())
}

/// Generate save analysis data of a crate to be used later by the RLS library later and load it
/// into the analysis host.
///
/// ## Arguments:
///
/// - `config`: Contains data for what needs to be output or used. In this case the path to the
///             `Cargo.toml` file
/// - `target`: The target to document
fn generate_and_load_analysis(config: &Config, target: &Target) -> Result<()> {
    let task = config.ui.start_task("Generating save analysis data");
    task.report("In progress");

    let analysis_result = cargo::generate_analysis(config, target, |progress| {
        task.report(progress);
    });

    if analysis_result.is_err() {
        task.error();
        return analysis_result;
    }

    drop(task);

    let task = config.ui.start_task("Loading save analysis data");
    task.report("In Progress");

    let root_path = config.root_path();
    config.host.reload(root_path, root_path)?;

    drop(task);

    Ok(())
}
