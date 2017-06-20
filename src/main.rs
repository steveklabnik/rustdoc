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
}

impl Config {
    fn new(matches: &clap::ArgMatches) -> Result<Config, Box<std::error::Error>> {
        // unwrap is okay because we take a default value
        let manifest_path = PathBuf::from(matches.value_of("manifest-path").unwrap());
        let host = generate_analysis(&manifest_path)?;

        Ok(Config {
            manifest_path,
            host,
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
        .subcommand(
            SubCommand::with_name("build").about("generates documentation"),
        )
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
    // TODO: write to the correct path
    //let output_path = PathBuf::from(format!("{}/target/doc", config.manifest_path.display()));
    //fs::create_dir_all(&output_path)?;
    let output_path = PathBuf::from(r".\frontend\public\data.json");

    use jsonapi::api::*;

    let mut document = JsonApiDocument::default();

    let mut map = HashMap::new();
    map.insert(String::from("docs"), serde_json::Value::String(String::from("zomg docs")));

    let relationship = Relationship {
        data: IdentifierData::Multiple(vec![ResourceIdentifier{
            _type: String::from("module"),
            id: String::from("example::foo"),
        }]),
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
    map.insert(String::from("name"), serde_json::Value::String(String::from("foo")));
    map.insert(String::from("docs"), serde_json::Value::String(String::from("oh boy\n\n*THIS*")));

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

    let mut file = File::create(output_path)?;
    file.write_all(serialized.as_bytes())?;

    println!("done.");

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
            )
                .into(),
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

