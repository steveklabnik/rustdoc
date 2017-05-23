extern crate clap;
extern crate rls_analysis as analysis;
use clap::{Arg, App, SubCommand};

use std::env;
use std::io;
use std::io::prelude::*;
use std::process::{Command, Stdio};

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let matches = App::new("rustdoc")
        .version(version)
        .author("Steve Klabnik <steve@steveklabnik.com>")
        .about("Generate web-based documentation from your Rust code.")
        .subcommand(SubCommand::with_name("build").about("generates documentation"))
        .get_matches();

    match matches.subcommand_name() {
        Some("build") => println!("build"),
        None => println!("build"),
        _ => println!("not build"),
    }

    std::process::exit(0);

    let mut command = Command::new("cargo");

    command.arg("build");

    command.env("RUSTFLAGS", "-Z save-analysis");
    command.env("CARGO_TARGET_DIR", "target/rls");

    // for now, just eat the output
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    print!("generating save analysis data...");
    io::stdout().flush().unwrap();

    command.spawn().unwrap().wait().unwrap();
    println!("done.");

    print!("loading save analysis data...");
    io::stdout().flush().unwrap();
    let host = analysis::AnalysisHost::new(analysis::Target::Debug);
    let dir = &env::current_dir().unwrap();
    host.reload(dir, dir, true).unwrap();
    println!("done.");

    let roots = host.def_roots().unwrap();

    let &(id, _) = roots
        .iter()
        .find(|&&(_, ref name)| name == "example")
        .unwrap();

    host.for_each_child_def(
            id, |_, def| {
                println!("{}", def.name);
                println!("{}", def.docs);
            }
        )
        .unwrap();

}
