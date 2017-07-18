extern crate jsonapi;
extern crate rls_analysis as analysis;
extern crate serde_json;

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

use analysis::raw::DefKind;
use jsonapi::api::JsonApiDocument;

#[derive(Debug)]
pub struct CrateErr {
    error: String,
}

impl CrateErr {
    pub fn new(crate_name: &str) -> CrateErr {
        CrateErr { error: format!("Crate not found: \"{}\"", crate_name) }
    }
}

impl fmt::Display for CrateErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.error)
    }
}

impl std::error::Error for CrateErr {
    fn description(&self) -> &str {
        &self.error
    }
}

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
    pub fn new(manifest_path: PathBuf) -> Result<Config, Box<std::error::Error>> {
        let host = generate_analysis(&manifest_path)?;

        let assets = vec![
            Asset {
                name: "crossdomain.xml",
                contents: include_str!("../frontend/dist/crossdomain.xml"),
            },
            Asset {
                name: "index.html",
                contents: include_str!("../frontend/dist/index.html"),
            },
            Asset {
                name: "robots.txt",
                contents: include_str!("../frontend/dist/robots.txt"),
            },
            Asset {
                name: "assets/frontend-c6c060f7a38307646632f4d86abb552b.js",
                contents: include_str!(
                    "../frontend/dist/assets/frontend-c6c060f7a38307646632f4d86abb552b.js"
                ),
            },
            Asset {
                name: "assets/frontend-d41d8cd98f00b204e9800998ecf8427e.css",
                contents: include_str!(
                    "../frontend/dist/assets/frontend-d41d8cd98f00b204e9800998ecf8427e.css"
                ),
            },
            Asset {
                name: "assets/vendor-12abafe454d5f3c9655736792567755d.js",
                contents: include_str!(
                    "../frontend/dist/assets/vendor-12abafe454d5f3c9655736792567755d.js"
                ),
            },
            Asset {
                name: "assets/vendor-d41d8cd98f00b204e9800998ecf8427e.css",
                contents: include_str!(
                    "../frontend/dist/assets/vendor-d41d8cd98f00b204e9800998ecf8427e.css"
                ),
            },
        ];

        Ok(Config {
            manifest_path,
            host,
            assets,
        })
    }
}

fn generate_json(config: &Config) -> Result<JsonApiDocument, Box<std::error::Error>> {
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
        _ => return Err(Box::new(CrateErr::new("example"))),
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

pub fn build(config: &Config, artifacts: &[&str]) -> Result<(), Box<std::error::Error>> {
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

fn create_asset_file(name: &str, path: &Path, data: &str) -> Result<(), Box<std::error::Error>> {
    let mut asset_path = path.to_path_buf();
    asset_path.push(name);

    let mut file = File::create(asset_path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}

fn generate_analysis(
    manifest_path: &Path,
) -> Result<analysis::AnalysisHost, Box<std::error::Error>> {
    let mut command = Command::new("cargo");

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
    let host = analysis::AnalysisHost::new(analysis::Target::Debug);
    host.reload(manifest_path, manifest_path, true)?;
    println!("done.");

    Ok(host)
}
