extern crate jsonapi;
extern crate rls_analysis as analysis;
extern crate serde_json;
#[macro_use]
extern crate error_chain;

pub mod error;
use error::*;

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

use analysis::raw::DefKind;
use jsonapi::api::JsonApiDocument;


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


pub fn generate_json(config: &Config) -> Result<JsonApiDocument> {
    generate_analysis(config)?;

    let roots = config.host.def_roots()?;

    // the list of built-in crates. not sure if we want to whitelist these or something?
    /*
    "rand", "collections", "std", "panic_unwind", "std_unicode",
    "alloc_system", "unwind", "core", "libc", "alloc", "panic_abort",
    "compiler_builtins"
    */

    // FIXME: this whole code shouldn't look for a precise crate.
    let id = roots.iter().find(|&&(_, ref name)| name == "example");
    let id = match id {
        Some(&(id, _)) => id,
        _ => return Err(ErrorKind::CrateErr("example").into()),
    };

    let root_def = config.host.get_def(id)?;

    let defs = config.host.for_each_child_def(id, |_, def| def.clone())?;

    let kinds = vec![
        DefKind::Mod,
        DefKind::Static,
        DefKind::Const,
        DefKind::Enum,
        DefKind::Struct,
        DefKind::Union,
        DefKind::Trait,
        DefKind::Function,
        DefKind::Macro,
    ];

    let mut data = BTreeMap::new();

    for kind in kinds {
        let key = format!("{:?}", kind);
        data.insert(key.clone(), Vec::new());

        for def in defs.iter().filter(|def| def.kind == kind) {
            // unwrap is okay here because we have filtered for the kind we inserted above
            data.get_mut(&key).unwrap().push(def.clone());
        }
    }

    use jsonapi::api::*;

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

    for def in &data["Mod"] {
        if let IdentifierData::Multiple(ref mut v) = relationship.data {
            v.push(ResourceIdentifier {
                _type: String::from("module"),
                id: def.qualname.clone(),
            });
        };
        let mut map = HashMap::new();
        map.insert(
            String::from("name"),
            serde_json::Value::String(def.name.clone()),
        );
        map.insert(
            String::from("docs"),
            serde_json::Value::String(def.docs.clone()),
        );

        let module = Resource {
            _type: String::from("module"),
            id: def.qualname.clone(),
            attributes: map,
            links: None,
            meta: None,
            relationships: None,
        };

        document.included.as_mut().map(|v| v.push(module));
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

    Ok(document)
}

pub fn build(config: &Config, artifacts: &[&str]) -> Result<()> {
    let output_path = config.manifest_path.join("target/doc");
    fs::create_dir_all(&output_path)?;

    let mut stdout = io::stdout();

    if artifacts.contains(&"json") {
        print!("generating JSON...");
        stdout.flush()?;

        let document = generate_json(&config)?;
        let serialized = serde_json::to_string(&document)?;

        let mut json_path = output_path.clone();
        json_path.push("data.json");

        let mut file = File::create(json_path)?;
        file.write_all(serialized.as_bytes())?;
        println!("done.");
    }

    // now that we've written out the data, we can write out the rest of it
    if artifacts.contains(&"assets") {
        print!("copying assets...");
        stdout.flush()?;

        let mut assets_path = output_path.clone();
        assets_path.push("assets");
        fs::create_dir_all(&assets_path)?;

        for asset in &config.assets {
            create_asset_file(asset.name, &output_path, asset.contents)?;
        }

        println!("done.");
    }

    Ok(())
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
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest_path.join("Cargo.toml"))
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", manifest_path.join("target/rls"));

    let mut stdout = io::stdout();

    print!("generating save analysis data...");
    stdout.flush()?;

    let output = command.output()?;

    if !output.status.success() {
        println!("");
        return Err(
            format!(
                "Cargo failed with status {}. stderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ).into(),
        );
    }
    println!("done.");

    print!("loading save analysis data...");
    stdout.flush()?;
    config.host.reload(manifest_path, manifest_path, true)?;
    println!("done.");

    Ok(())
}
