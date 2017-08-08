#![warn(missing_docs)]

//! Code used to drive the creation of documentation for Rust Crates.

extern crate rustdoc;
extern crate clap;

use clap::{App, Arg, SubCommand};

use rustdoc::{Config, build};
use rustdoc::cargo;
use rustdoc::error::Result;

use std::io::{Write, stderr};
use std::process;
use std::path::PathBuf;

static ALL_ARTIFACTS: &[&str] = &["assets", "json"];

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let joined_artifacts = ALL_ARTIFACTS.join(",");
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
            SubCommand::with_name("build")
                .about("generates documentation")
                .arg(
                    Arg::with_name("artifacts")
                        .long("emit")
                        .use_delimiter(true)
                        .takes_value(true)
                        .possible_values(ALL_ARTIFACTS)
                        .default_value(&joined_artifacts)
                        .help("Build artifacts to produce. Defaults to everything."),
                ),
        )
        .get_matches();

    // unwrap is okay because we take a default value
    let manifest_path = PathBuf::from(&matches.value_of("manifest-path").unwrap());
    let assets = include!(concat!(env!("OUT_DIR"), "/asset.in"));

    let paths = cargo::workspace_members_from_metadata(&manifest_path).unwrap_or_else(|err| {
        println!("Problem getting workspace_members: {}", err);
        process::exit(1);
    });
    let mut configs = Vec::new();

    for path in paths.into_iter().map(|x| PathBuf::from(x)) {
        configs.push(Config::new(path, assets.clone()).unwrap_or_else(|err| {
            println!("Problem creating configuration: {}", err);
            process::exit(1);
        }));
    }

    let result = move | | -> Result<()> {
        match matches.subcommand() {
            ("build", Some(matches)) => {
                let artifacts: Vec<&str> = matches.values_of("artifacts").unwrap().collect();
                for config in configs {
                    build(&config, &artifacts)?;
                }
                Ok(())
            }
            // default is to build
            _ => {
                for config in configs {
                    build(&config, ALL_ARTIFACTS)?;
                }
                Ok(())
            }
        }
    };

    if let Err(ref e) = result() {
        let stderr = &mut stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "Error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "Caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "Backtrace: {:?}", backtrace).expect(errmsg);
        }

        process::exit(1);
    }
}
