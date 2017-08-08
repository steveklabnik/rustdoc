//! Functions used to generate the documentation for Rust Crates.

#![warn(missing_docs)]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;

extern crate indicatif;
extern crate rayon;
extern crate rls_analysis as analysis;
extern crate serde;

pub mod assets;
pub mod cargo;
pub mod error;
pub mod json;

pub use json::create_json;

use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;

use indicatif::ProgressBar;

use assets::Asset;
use cargo::Target;
use error::*;

pub use error::{Error, ErrorKind};

/// A structure that contains various fields that hold data in order to generate doc output.
pub struct Config {
    /// Path to the directory with the `Cargo.toml` file for the crate being analyzed
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
    /// - `manifest_path`: The path to the location of `Cargo.toml` of the crate being documented
    pub fn new(manifest_path: PathBuf, assets: Vec<Asset>) -> Result<Config> {
        let host = analysis::AnalysisHost::new(analysis::Target::Debug);

        Ok(Config {
            manifest_path,
            host,
            assets,
        })
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
    let target = cargo::target_from_metadata(&metadata)?;
    generate_and_load_analysis(config, &target)?;

    let output_path = config.manifest_path.join("target/doc");
    fs::create_dir_all(&output_path)?;

    if artifacts.contains(&"json") {
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(50);
        spinner.set_message("Generating JSON: In Progress");

        let json = create_json(&config.host, &target.crate_name())?;

        let mut json_path = output_path.clone();
        json_path.push("data.json");

        let mut file = File::create(json_path)?;
        file.write_all(json.as_bytes())?;
        spinner.finish_with_message("Generating JSON: Done");
    }

    // now that we've written out the data, we can write out the rest of it
    if artifacts.contains(&"assets") {
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(50);
        spinner.set_message("Copying Assets: In Progress");

        let mut assets_path = output_path.clone();
        assets_path.push("assets");
        fs::create_dir_all(&assets_path)?;

        for asset in &config.assets {
            assets::create_asset_file(asset.name, &output_path, asset.contents)?;
        }

        spinner.finish_with_message("Copying Assets: Done");
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
    let manifest_path = &config.manifest_path;

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(50);
    spinner.set_message("Generating save analysis data: In Progress");

    if let Err(e) = cargo::generate_analysis(manifest_path, target) {
        spinner.finish_with_message("Generating save analysis data: Error");
        return Err(e);
    }

    spinner.finish_with_message("Generating save analysis data: Done");

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(50);
    spinner.set_message("Loading save analysis data: In Progress");
    config.host.reload(manifest_path, manifest_path)?;
    spinner.finish_with_message("Loading save analysis data: Done");

    Ok(())
}
