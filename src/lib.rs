extern crate jsonapi;
extern crate rls_analysis as analysis;
extern crate serde_json;
#[macro_use]
extern crate error_chain;
extern crate indicatif;

pub mod error;
use error::*;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

use analysis::raw::DefKind;
use indicatif::ProgressBar;

pub struct Config {
    manifest_path: PathBuf,
    host: analysis::AnalysisHost,
    assets: Vec<Asset>,
}

/// Static assets compiled into the binary so we get a single executable.
///
/// In the future I expect these to be Cow<'static, str>s to support dynamic assets
struct Asset {
    name: &'static str,
    contents: &'static str,
}

impl Config {
    pub fn new(manifest_path: PathBuf) -> Result<Config> {
        let host = analysis::AnalysisHost::new(analysis::Target::Debug);

        let assets = include!("asset.in");

        Ok(Config {
            manifest_path,
            host,
            assets,
        })
    }
}

pub fn build(config: &Config, artifacts: &[&str]) -> Result<()> {
    generate_analysis(config)?;

    let data = DocData::new(config)?;

    let output_path = config.manifest_path.join("target/doc");
    fs::create_dir_all(&output_path)?;

    if artifacts.contains(&"json") {
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(50);
        spinner.set_message("Generating JSON: In Progress");

        let json = data.to_json(config)?;

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
            create_asset_file(asset.name, &output_path, asset.contents)?;
        }

        spinner.finish_with_message("Copying Assets: Done");
    }

    Ok(())
}

fn package_name_from_manifest_path(manifest_path: &Path) -> Result<String> {
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

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    match json["packages"][0]["name"] {
        serde_json::Value::String(ref name) => Ok(name.clone()),
        _ => Err(ErrorKind::Json("cargo metadata").into()),
    }
}

fn create_asset_file(name: &str, path: &Path, data: &str) -> Result<()> {
    let mut asset_path = path.to_path_buf();
    asset_path.push(name);

    let mut file = File::create(asset_path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}

fn generate_analysis(config: &Config) -> Result<()> {
    let mut command = Command::new("cargo");
    let manifest_path = &config.manifest_path;

    command
        .arg("check")
        .arg("--manifest-path")
        .arg(manifest_path.join("Cargo.toml"))
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", manifest_path.join("target/rls"));

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(50);
    spinner.set_message("Generating save analysis data: In Progress");

    let output = command.output()?;

    if !output.status.success() {
        spinner.finish_with_message("Generating save analysis data: Error");
        return Err(
            ErrorKind::Cargo(
                output.status,
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ).into(),
        );
    }
    spinner.finish_with_message("Generating save analysis data: Done");

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(50);
    spinner.set_message("Loading save analysis data: In Progress");
    config.host.reload(manifest_path, manifest_path, true)?;
    spinner.finish_with_message("Loading save analysis data: Done");

    Ok(())
}

#[derive(Debug)]
struct DocData {
    krate: Crate,
    data: HashMap<String, Item>,
}

impl DocData {
    fn new(config: &Config) -> Result<DocData> {
        let roots = config.host.def_roots()?;

        let package = package_name_from_manifest_path(&config.manifest_path)?;
        let id = roots.iter().find(|&&(_, ref name)| name == &package);
        let root_id = match id {
            Some(&(id, _)) => id,
            _ => return Err(ErrorKind::CrateErr("example").into()),
        };

        let root_def = config.host.get_def(root_id)?;

        let name_len = root_def.qualname.len();
        let mut krate = Crate {
            id: root_id,
            // example:: -> example
            name: root_def.qualname[..(name_len - 2)].to_string(),
            docs: root_def.docs.clone(),
            modules: Vec::new(),
        };

        let data = DocData::build_data(config, root_id, &mut krate)?;

        Ok(DocData { krate, data })
    }

    fn build_data(
        config: &Config,
        root_id: analysis::Id,
        krate: &mut Crate,
    ) -> Result<HashMap<String, Item>> {
        let mut data = HashMap::new();

        let defs = config.host.for_each_child_def(
            root_id,
            |_, def| def.clone(),
        )?;

        for def in defs.iter() {
            match def.kind {
                DefKind::Mod => {
                    data.insert(
                        def.qualname.clone(),
                        Item::Module {
                            name: def.name.clone(),
                            docs: def.docs.clone(),
                        },
                    );
                    krate.modules.push(def.qualname.clone());
                }
                DefKind::Static => (),
                DefKind::Const => (),
                DefKind::Enum => (),
                DefKind::Struct => (),
                DefKind::Union => (),
                DefKind::Trait => (),
                DefKind::Function => (),
                DefKind::Macro => (),
                DefKind::Tuple => (),
                DefKind::Method => (),
                DefKind::Type => (),
                DefKind::Local => (),
                DefKind::Field => (),
            }
        }

        Ok(data)
    }

    fn to_json(&self, config: &Config) -> Result<String> {
        use jsonapi::api::*;

        let root_def = config.host.get_def(self.krate.id)?;

        let mut document = JsonApiDocument::default();

        let mut map = HashMap::new();
        map.insert(
            String::from("docs"),
            serde_json::Value::String(root_def.docs.clone()),
        );

        let mut relationships = HashMap::new();

        let mut relationship = Relationship {
            data: IdentifierData::Multiple(Vec::new()),
            links: None,
        };

        //TODO this is bad, use real option handling in the loop
        document.included = Some(Vec::new());

        for (id, item) in self.data.iter() {
            match item {
                &Item::Module { ref name, ref docs } => {
                    if let IdentifierData::Multiple(ref mut v) = relationship.data {
                        v.push(ResourceIdentifier {
                            _type: String::from("module"),
                            id: id.clone(),
                        });
                    };
                    let mut map = HashMap::new();
                    map.insert(
                        String::from("name"),
                        serde_json::Value::String(name.clone()),
                    );
                    map.insert(
                        String::from("docs"),
                        serde_json::Value::String(docs.clone()),
                    );

                    let module = Resource {
                        _type: String::from("module"),
                        id: id.clone(),
                        attributes: map,
                        links: None,
                        meta: None,
                        relationships: None,
                    };

                    document.included.as_mut().map(|v| v.push(module));
                }
            }
        }

        relationships.insert(String::from("modules"), relationship);

        let len = root_def.qualname.len();
        let krate = Resource {
            _type: String::from("crate"),
            // example:: -> example
            id: root_def.qualname[..(len - 2)].to_string(),
            attributes: map,
            links: None,
            meta: None,
            relationships: Some(relationships),
        };

        document.data = Some(PrimaryData::Single(Box::new(krate)));

        Ok(serde_json::to_string(&document)?)
    }
}

#[derive(Debug)]
struct Crate {
    id: analysis::Id,
    name: String,
    docs: String,
    modules: Vec<String>,
}

#[derive(Debug)]
enum Item {
    Module { name: String, docs: String },
}
