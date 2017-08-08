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

use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;

use analysis::AnalysisHost;
use analysis::DefKind;
use indicatif::ProgressBar;
use rayon::prelude::*;

use assets::Asset;
use cargo::Target;
use error::*;
use json::*;

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

/// This creates the JSON documentation from the given `AnalysisHost`.
pub fn create_json(host: &AnalysisHost, crate_name: &str) -> Result<String> {
    let roots = host.def_roots()?;

    let id = roots.iter().find(|&&(_, ref name)| name == crate_name);
    let root_id = match id {
        Some(&(id, _)) => id,
        _ => return Err(ErrorKind::CrateErr(crate_name.to_string()).into()),
    };

    let root_def = host.get_def(root_id)?;

    fn recur(id: &analysis::Id, host: &AnalysisHost) -> Result<Vec<Document>> {
        let def = host.get_def(*id)?;

        let mut ids = Vec::new();
        host.for_each_child_def(*id, |id, _| ids.push(id))?;

        let ty = match def.kind {
            DefKind::Mod => String::from("module"),
            DefKind::Struct => String::from("struct"),
            _ => String::from("oh no"),
        };

        // Using the item's metadata we create a new `Document` type to be put in the eventual
        // serialized JSON.
        let mut doc = Document::new()
            .ty(ty.clone())
            .id(def.qualname.clone())
            .attributes(String::from("name"), def.name)
            .attributes(String::from("docs"), def.docs);

        let mut docs = Vec::new();

        let child_docs: Vec<Document> = ids.into_par_iter()
            .map(|id: analysis::Id| recur(&id, host))
            .reduce(|| Ok(Vec::new()), |a: Result<Vec<Document>>,
             b: Result<Vec<Document>>| {
                let mut a = a?;

                a.extend(b?);

                Ok(a)
            })?;

        // create child relationships
        let relationships: Vec<_> = child_docs
            .par_iter()
            .filter(|doc| doc.ty == "module")
            .map(|doc| Data::new().ty(doc.ty.clone()).id(doc.id.clone()))
            .collect();

        // if we have no children, no need to set a relationship
        if relationships.len() > 0 {
            doc.relationships(String::from("child_modules"), relationships);
        }
        docs.push(doc);

        docs.extend(child_docs);
        Ok(docs)
    }

    let mut included = recur(&root_id, host)?;

    // first one is for the crate, which we don't need
    included.remove(0);

    let mut crate_document = Document::new()
        .ty(String::from("crate"))
        .id(crate_name.to_string())
        .attributes(String::from("docs"), root_def.docs);

    // set up relationships for the crate
    host.for_each_child_def(root_id, |_, def| {
        let (ty, relations_key) = match def.kind {
            DefKind::Mod => (String::from("module"), String::from("modules")),
            DefKind::Struct => (String::from("struct"), String::from("structs")),
            _ => return,
        };

        crate_document.relationships(
            relations_key,
            vec![Data::new().ty(ty).id(def.qualname.clone())],
        );
    }).unwrap();

    Ok(serde_json::to_string(
        &Documentation::new().data(crate_document).included(
            included,
        ),
    )?)
}
