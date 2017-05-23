extern crate clap;
extern crate rls_analysis as analysis;
use clap::{App, SubCommand};

use std::env;
use std::io;
use std::io::prelude::*;
use std::process::{self, Command, Stdio};

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let matches = App::new("rustdoc")
        .version(version)
        .author("Steve Klabnik <steve@steveklabnik.com>")
        .about("Generate web-based documentation from your Rust code.")
        .subcommand(SubCommand::with_name("build").about("generates documentation"))
        .get_matches();

    let result = match matches.subcommand_name() {
        Some("build") => build(),
        None => build(),
        _ => unreachable!(),
    };

    if let Err(e) = result {
        println!("Application error: {}", e);

        process::exit(1);
    }
}

fn build() -> Result<(), Box<std::error::Error>> {
    let host = generate_analysis()?;

    let roots = host.def_roots().unwrap();

    let &(id, _) = roots
        .iter()
        .find(|&&(_, ref name)| name == "example")
        .unwrap();

    println!("root elements of this crate:");

    host.for_each_child_def(
            id, |_, def| {
                println!("{}", def.name);
            }
        )
        .unwrap();

    Ok(())
}

fn generate_analysis() -> Result<analysis::AnalysisHost, Box<std::error::Error>> {
    let mut command = Command::new("cargo");

    command.arg("build");

    command.env("RUSTFLAGS", "-Z save-analysis");
    command.env("CARGO_TARGET_DIR", "target/rls");

    // for now, just eat the output
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    print!("generating save analysis data...");
    io::stdout().flush()?;

    command.spawn()?.wait()?;
    println!("done.");

    print!("loading save analysis data...");
    io::stdout().flush()?;
    let host = analysis::AnalysisHost::new(analysis::Target::Debug);
    let dir = &env::current_dir()?;
    host.reload(dir, dir, true).unwrap();
    println!("done.");

    Ok(host)
}
