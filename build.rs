use std::process::Command;

fn main() {
    let mut output = Command::new(r"C:\Program Files (x86)\Nodist\bin\ember.cmd");

    output.current_dir("frontend");

    output.arg("build");
    output.arg("--prod");

    output.arg("-o");
    output.arg("../target/frontend");

    let output = output.output().unwrap();

    if !output.status.success() {
        panic!(
            "build script failed with status {}. stderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("cargo:rerun-if-changed=frontend")
}
