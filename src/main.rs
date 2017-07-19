extern crate rustdoc;
extern crate clap;

use clap::{App, Arg, SubCommand};

use rustdoc::{Config, build};

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
    let config = Config::new(manifest_path).unwrap_or_else(|err| {
        println!("Problem creating configuration: {}", err);
        process::exit(1);
    });

    let result = match matches.subcommand() {
        ("build", Some(matches)) => {
            let artifacts: Vec<&str> = matches.values_of("artifacts").unwrap().collect();
            build(&config, &artifacts)
        }
        // default is to build
        _ => build(&config, ALL_ARTIFACTS),
    };

    if let Err(ref e) = result {
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
