extern crate rustdoc;
extern crate clap;

use clap::{App, Arg, SubCommand};

use rustdoc::{Config, BuildOutput, build};

use std::process;
use std::path::PathBuf;

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
            SubCommand::with_name("build")
                .about("generates documentation")
                .arg(
                    Arg::with_name("json")
                        .long("json")
                        .short("j")
                        .takes_value(false)
                        .help("Only output JSON from rustdoc"),
                ),
        )
        .get_matches();

    // unwrap is okay because we take a default value
    let manifest_path = PathBuf::from(&matches.value_of("manifest-path").unwrap());

    let build_output = match matches.subcommand_matches("build") {
        Some(j) => {
            if j.is_present("json") {
                BuildOutput::Json
            } else {
                BuildOutput::All
            }
        }
        None => BuildOutput::All,
    };

    let config = Config::new(manifest_path, build_output).unwrap_or_else(|err| {
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
