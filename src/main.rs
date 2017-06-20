extern crate clap;
extern crate jsonapi;
extern crate rls_analysis as analysis;
extern crate serde_json;

use analysis::raw::DefKind;

use clap::{App, Arg, SubCommand};

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

struct Config {
    manifest_path: PathBuf,
    host: analysis::AnalysisHost,
    assets: Assets,
}

/// Static assets compiled into the binary so we get a single executable.
///
/// In the future I expect these to be Cow<'static, str>s to support dynamic assets
struct Assets {
    crossdomain_xml: &'static str,
    index_html: &'static str,
    robots_txt: &'static str,
    frontend_js: &'static str,
    frontend_css: &'static str,
    vendor_js: &'static str,
    vendor_css: &'static str,
}

impl Config {
    fn new(matches: &clap::ArgMatches) -> Result<Config, Box<std::error::Error>> {
        // unwrap is okay because we take a default value
        let manifest_path = PathBuf::from(matches.value_of("manifest-path").unwrap());
        let host = generate_analysis(&manifest_path)?;

        // TODO: stop being so hilariously hard-coded
        let assets = Assets {
            crossdomain_xml: include_str!("../frontend/dist/crossdomain.xml"),
            index_html: include_str!("../frontend/dist/index.html"),
            robots_txt: include_str!("../frontend/dist/robots.txt"),
            frontend_js: include_str!(
                "../frontend/dist/assets/frontend-0da8276493f72a2ba7d806a3de281626.js"
            ),
            frontend_css: include_str!(
                "../frontend/dist/assets/frontend-d41d8cd98f00b204e9800998ecf8427e.css"
            ),
            vendor_js: include_str!(
                "../frontend/dist/assets/vendor-315855171bdc0bd4fe4df9973f5d2ece.js"
            ),
            vendor_css: include_str!(
                "../frontend/dist/assets/vendor-d41d8cd98f00b204e9800998ecf8427e.css"
            ),
        };

        Ok(Config {
            manifest_path,
            host,
            assets,
        })
    }
}

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let matches = App::new("rustdoc")
        .version(version)
        .author("Steve Klabnik <steve@steveklabnik.com>")
        .about("Generate web-based documentation from your Rust code.")
        .arg(
            Arg::with_name("manifest-path")
                .long("manifest-path")
                // remove the unwrap in Config::new if this default_value goes away
                .default_value(".")
                .help("The path to the Cargo manifest of the project you are documenting."),
        )
        .subcommand(SubCommand::with_name("build").about(
            "generates documentation",
        ))
        .get_matches();

    let config = Config::new(&matches).unwrap_or_else(|err| {
        println!("Problem creating configuration: {}", err);
        process::exit(1);
    });

    let result = match matches.subcommand_name() {
        Some("build") => build(&config),
        // default is to build
        None => build(&config),
        Some(_) => Err(
            "Something strange is going on with subcommands, please file a bug!".into(),
        ),
    };

    if let Err(e) = result {
        println!("Application error: {}", e);

        process::exit(1);
    }
}

fn build(config: &Config) -> Result<(), Box<std::error::Error>> {
    print!("generating JSON...");
    io::stdout().flush()?;

    let roots = config.host.def_roots()?;

    // the list of built-in crates. not sure if we want to whitelist these or something?
    /* 
    "rand", "collections", "std", "panic_unwind", "std_unicode",
    "alloc_system", "unwind", "core", "libc", "alloc", "panic_abort",
    "compiler_builtins"
    */

    let &(id, _) = roots
        .iter()
        .find(|&&(_, ref name)| name == "example")
        .unwrap();

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
            data.get_mut(&key).unwrap().push(def.name.clone());
        }
    }

    // TODO: use real fs handling here
    let output_path = PathBuf::from(format!("{}/target/doc", config.manifest_path.display()));
    fs::create_dir_all(&output_path)?;

    let mut json_path = output_path.clone();
    json_path.push("data.json");

    use jsonapi::api::*;

    let mut document = JsonApiDocument::default();

    let mut map = HashMap::new();
    map.insert(
        String::from("docs"),
        serde_json::Value::String(String::from("zomg docs")),
    );

    let relationship = Relationship {
        data: IdentifierData::Multiple(vec![
            ResourceIdentifier {
                _type: String::from("module"),
                id: String::from("example::foo"),
            },
        ]),
        links: None,
    };

    let mut relationships = HashMap::new();
    relationships.insert(String::from("modules"), relationship);

    let krate = Resource {
        _type: String::from("crate"),
        id: String::from("example"),
        attributes: map,
        links: None,
        meta: None,
        relationships: Some(relationships),
    };

    document.data = Some(PrimaryData::Single(Box::new(krate)));

    let mut map = HashMap::new();
    map.insert(
        String::from("name"),
        serde_json::Value::String(String::from("foo")),
    );
    map.insert(
        String::from("docs"),
        serde_json::Value::String(String::from("oh boy\n\n*THIS*")),
    );

    let module = Resource {
        _type: String::from("module"),
        id: String::from("example::foo"),
        attributes: map,
        links: None,
        meta: None,
        relationships: None,
    };

    document.included = Some(vec![module]);

    let serialized = serde_json::to_string(&document)?;

    let mut file = File::create(json_path)?;
    file.write_all(serialized.as_bytes())?;

    // now that we've written out the data, we can write out the rest of it
    create_asset_file(
        "crossdomain.xml",
        &output_path,
        config.assets.crossdomain_xml,
    )?;
    create_asset_file("index.html", &output_path, config.assets.index_html)?;
    create_asset_file("robots.txt", &output_path, config.assets.robots_txt)?;
    create_asset_file(
        "crossdomain.xml",
        &output_path,
        config.assets.crossdomain_xml,
    )?;

    let mut assets_path = output_path.clone();
    assets_path.push("assets");
    fs::create_dir_all(&assets_path)?;

    // TODO: stop being so hilariously hard-coded
    create_asset_file(
        "frontend-0da8276493f72a2ba7d806a3de281626.js",
        &assets_path,
        config.assets.frontend_js,
    )?;
    create_asset_file(
        "frontend-d41d8cd98f00b204e9800998ecf8427e.css",
        &assets_path,
        config.assets.frontend_css,
    )?;
    create_asset_file(
        "vendor-315855171bdc0bd4fe4df9973f5d2ece.js",
        &assets_path,
        config.assets.vendor_js,
    )?;
    create_asset_file(
        ".vendor-d41d8cd98f00b204e9800998ecf8427e.css",
        &assets_path,
        config.assets.vendor_css,
    )?;

    println!("done.");

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

    let manifest_path = manifest_path.to_str().unwrap();

    command.arg("build");
    // TODO build an actual path
    command.args(
        &["--manifest-path", &format!("{}/Cargo.toml", manifest_path)],
    );
    command.env("RUSTFLAGS", "-Z save-analysis");
    // TODO build an actual path
    command.env("CARGO_TARGET_DIR", &format!("{}/target/rls", manifest_path));

    print!("generating save analysis data...");
    io::stdout().flush()?;

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
    io::stdout().flush()?;
    let host = analysis::AnalysisHost::new(analysis::Target::Debug);
    host.reload(
        &PathBuf::from(manifest_path),
        &PathBuf::from(manifest_path),
        true,
    )?;
    println!("done.");

    Ok(host)
}
