extern crate rls_analysis as analysis;

use std::env;
use std::process::Command;

fn main() {
    let mut command = Command::new("cargo");

    command.arg("build");

    command.env("RUSTFLAGS", "-Z save-analysis");
    command.env("CARGO_TARGET_DIR", "target/rls");

    println!("running {:?}", command);
    command.spawn().unwrap().wait().unwrap();
    println!("done");

    let host = analysis::AnalysisHost::new(analysis::Target::Debug);
    let dir = &env::current_dir().unwrap();
    host.reload(dir, dir, true).unwrap();

    let roots = host.def_roots().unwrap();

    //println!("Found these roots: {:?}", roots);

    let &(id, _) = roots.iter().find(|&&(id, ref name)| {
        println!("{}, {}", id, name);
        name == "example"
    }).unwrap();

    println!("id {}", id);

    println!("has def {}", host.has_def(id));

    host.for_each_child_def(id, |id, def| {
        println!("id {}, def {:?}", id, def);
    }).unwrap();
    
}
