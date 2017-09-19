//! Functions used to generate the documentation for Rust Crates.

#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;

extern crate indicatif;
extern crate open;
extern crate rls_analysis as analysis;

pub mod assets;
pub mod cargo;
pub mod error;
pub mod json;

mod ui;

pub use json::create_json;

use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use assets::Asset;
use cargo::Target;
use error::*;
use ui::Ui;

pub use error::{Error, ErrorKind};
pub use ui::Verbosity;

/// A structure that contains various fields that hold data in order to generate doc output.
#[derive(Debug)]
pub struct Config {
    /// Interactions with the user interface.
    ui: Ui,

    /// Path to the `Cargo.toml` file for the crate being analyzed
    manifest_path: PathBuf,

    /// Contains the Cargo analysis output for the crate being documented
    host: analysis::AnalysisHost,

    /// Contains all of the `Asset`s that will be output at the end (e.g. JSON, CSS, HTML and/or
    /// JS)
    assets: Vec<Asset>,
}

impl Config {
    /// Create a new `Config` based off the location of the manifest as well as assets generated
    /// during the build phase
    ///
    /// ## Arguments
    ///
    /// - `manifest_path`: The path to the `Cargo.toml` of the crate being documented
    pub fn new(verbosity: Verbosity, manifest_path: PathBuf, assets: Vec<Asset>) -> Result<Config> {
        let host = analysis::AnalysisHost::new(analysis::Target::Debug);

        if !manifest_path.is_file() || !manifest_path.ends_with("Cargo.toml") {
            bail!("The --manifest-path must be a path to a Cargo.toml file");
        }

        Ok(Config {
            ui: Ui::new(verbosity),
            manifest_path,
            host,
            assets,
        })
    }

    /// Returns the directory containing the `Cargo.toml` of the crate being documented.
    pub fn root_path(&self) -> &Path {
        // unwrap() is safe, as manifest_path will point to a file
        self.manifest_path.parent().unwrap()
    }

    /// Returns the directory where output files should be placed
    pub fn output_path(&self) -> PathBuf {
        self.root_path().join("target").join("doc")
    }

    /// Open the generated docs in a web browser.
    pub fn open_docs(&self) -> Result<()> {
        open::that(self.output_path().join("index.html"))?;
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

    if artifacts.contains(&"json") {
        let task = config.ui.start_task("Generating JSON");
        task.report("In Progress");

        let json = create_json(&config.host, &target.crate_name())?;

        let json_path = output_path.join("data.json");
        let mut file = File::create(json_path)?;
        file.write_all(json.as_bytes())?;
    }

    // now that we've written out the data, we can write out the rest of it
    if artifacts.contains(&"assets") {
        let task = config.ui.start_task("Copying Assets");
        task.report("In Progress");

        let assets_path = output_path.join("assets");
        fs::create_dir_all(&assets_path)?;

        for asset in &config.assets {
            assets::create_asset_file(asset.name, &output_path, asset.contents)?;
        }
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

    let analysis_result =
        cargo::generate_analysis(config, target, |progress| { task.report(progress); });

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
