extern crate rls_analysis as analysis;

use std::env;
use std::io;
use std::io::prelude::*;
use std::process::{Command, Stdio};

fn main() {
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

    let &(id, _) = roots.iter().find(|&&(_, ref name)| {
        name == "example"
    }).unwrap();

    host.for_each_child_def(id, |_, def| {
        println!("{}", def.name);
        println!("{}", def.docs);
    }).unwrap();
    
}
