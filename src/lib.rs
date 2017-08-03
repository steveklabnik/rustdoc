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

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;

use analysis::AnalysisHost;
use analysis::raw::DefKind;
use indicatif::ProgressBar;
use rayon::prelude::*;

use assets::Asset;
use error::*;
use json::*;

pub use error::{Error, ErrorKind};

/// A structure that contains various fields that hold data in order to generate doc output.
///
/// ## Fields
///
/// - `manifest_path`: Path to the directory with the `Cargo.toml` file for the crate being analyzed
/// - `host`: Contains the Cargo analysis output for the crate being documented
/// - `assets`: Contains all of the `Asset`s that will be output at the end (e.g. JSON, CSS, HTML
///             and/or JS)
pub struct Config {
    manifest_path: PathBuf,
    host: analysis::AnalysisHost,
    assets: Vec<Asset>,
}

impl Config {
    /// Create a new `Config` based off the location of the manifest as well as assets generated
    /// during the build phase
    ///
    /// ## Arguments
    ///
    /// - manifest_path: The path to the location of `Cargo.toml` of the crate being documented
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
/// - config: The `Config` struct that contains the data needed to generate the documentation
/// - artifacts: A slice containing what assets should be output at the end
pub fn build(config: &Config, artifacts: &[&str]) -> Result<()> {
    generate_and_load_analysis(config)?;

    let crate_name = cargo::crate_name_from_manifest_path(&config.manifest_path)?;

    let output_path = config.manifest_path.join("target/doc");
    fs::create_dir_all(&output_path)?;

    if artifacts.contains(&"json") {
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(50);
        spinner.set_message("Generating JSON: In Progress");

        let json = create_json(&config.host, &crate_name)?;

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
/// - config: Contains data for what needs to be output or used. In this case the path to the
///           `Cargo.toml` file
fn generate_and_load_analysis(config: &Config) -> Result<()> {
    let manifest_path = &config.manifest_path;

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(50);
    spinner.set_message("Generating save analysis data: In Progress");

    if let Err(e) = cargo::generate_analysis(manifest_path) {
        spinner.finish_with_message("Generating save analysis data: Error");
        return Err(e);
    }

    spinner.finish_with_message("Generating save analysis data: Done");

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(50);
    spinner.set_message("Loading save analysis data: In Progress");
    config.host.reload(manifest_path, manifest_path, true)?;
    spinner.finish_with_message("Loading save analysis data: Done");

    Ok(())
}

/// This creates the JSON documentation from the given AnalysisHost.
pub fn create_json(host: &AnalysisHost, crate_name: &str) -> Result<String> {
    let roots = host.def_roots()?;

    let id = roots.iter().find(|&&(_, ref name)| name == &crate_name);
    let root_id = match id {
        Some(&(id, _)) => id,
        _ => return Err(ErrorKind::CrateErr(crate_name.to_string()).into()),
    };

    let root_def = host.get_def(root_id)?;

    fn recur(id: &analysis::Id, host: &AnalysisHost) -> Vec<analysis::Def> {
        let mut ids = Vec::new();
        let mut defs = host.for_each_child_def(*id, |id, def| {
            ids.push(id);
            def.clone()
        }).unwrap();

        let child_defs: Vec<analysis::Def> = ids.into_par_iter()
            .map(|id: analysis::Id| recur(&id, host))
            .reduce(|| Vec::new(), |mut a: Vec<analysis::Def>,
             b: Vec<analysis::Def>| {
                a.extend(b);
                a
            });
        defs.extend(child_defs);
        defs
    }

    let mut included: Vec<Document> = Vec::new();
    let mut relationships: HashMap<String, Vec<Data>> = HashMap::with_capacity(METADATA_SIZE);

    for def in recur(&root_id, host) {
        let (ty, relations_key) = match def.kind {
            DefKind::Mod => (String::from("module"), String::from("modules")),
            DefKind::Struct => (String::from("struct"), String::from("structs")),
            _ => continue,
        };

        // Using the item's metadata we create a new `Document` type to be put in the eventual
        // serialized JSON.
        included.push(
            Document::new()
                .ty(ty.clone())
                .id(def.qualname.clone())
                .attributes(String::from("name"), def.name)
                .attributes(String::from("docs"), def.docs),
        );

        let item_relationships = relationships.entry(relations_key).or_insert_with(
            Default::default,
        );
        item_relationships.push(Data::new().ty(ty).id(def.qualname));
    }

    let mut data_document = Document::new()
        .ty(String::from("crate"))
        .id(crate_name.to_string())
        .attributes(String::from("docs"), root_def.docs);

    // Insert all of the different types of relationships into this `Document` type only
    for (ty, data) in relationships.into_iter() {
        data_document.relationships(ty, data);
    }

    Ok(serde_json::to_string(
        &Documentation::new().data(data_document).included(
            included,
        ),
    )?)
}
