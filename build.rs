//! This build script creates source tests from the files in the `tests/source` directory.

#![recursion_limit = "1024"]

#[macro_use]
extern crate quote;

use std::env;
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::io;
use std::path::Path;
use std::process;

use quote::Ident;

const SOURCE_TEST_DIR: &str = "tests/source";

fn run() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut generated_code = File::create(Path::new(&out_dir).join("source_generated.rs"))?;

    let source_dir = Path::new(SOURCE_TEST_DIR);
    for source_file in fs::read_dir(&source_dir)? {
        let source_file = source_file?.path();

        let is_rust_file = source_file
            .extension()
            .map(|ext| ext == "rs")
            .unwrap_or_default();
        if !is_rust_file {
            continue;
        }

        let test_name = Ident::new(
            source_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("invalid file stem"),
        );

        let source_file_path = source_file.to_str().unwrap();

        let test =
            quote! {
            #[test]
            fn #test_name() {
                use std::env;
                use tempdir::TempDir;

                let tempdir = TempDir::new("rustdoc-test").unwrap();
                let source_file = env::current_dir().unwrap().join(#source_file_path);
                let host = ::generate_analysis(&source_file, tempdir.path()).unwrap();
                if let Err(err) = ::check(&source_file, &host) {
                    println!("error: {}", err);

                    println!("caused by: {}", err.cause());

                    println!("backtrace, if any: {:?}", err.backtrace());

                    panic!();
                }
            }
        };

        write!(generated_code, "{}", test.as_str())?;
    }

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
