#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate quote;
extern crate syn;
extern crate glob;

use std::env;
use std::fmt;
use std::fs::File;
use std::fs;
use std::io::{Write, BufWriter, stderr};
use std::path::Path;
use std::process::exit;
use std::thread;

use quote::Ident;
use glob::glob;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        PatternError(::glob::PatternError);
        GlobError(::glob::GlobError);
    }
}

const SOURCE_TEST_DIR: &str = "tests/source";

// Location of the dist folder
const DIST: &str = "frontend/dist/";

fn run() -> Result<()> {
    let source_thread = thread::spawn(|| generate_source_tests());
    let asset_thread = thread::spawn(|| write_asset_file());

    source_thread.join().unwrap()?;
    asset_thread.join().unwrap()?;

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
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

        exit(1);
    }
}

/// Start the recursion from the top level of the frontend's dist folder
fn acquire_assets() -> Result<Vec<Asset>> {
    let mut output: Vec<Asset> = Vec::new();
    let whitelist = vec![
        "assets/**/*",
        "crossdomain.xml",
        "ember-fetch/**/*",
        "index.html",
        "robots.txt",
    ];

    for w in whitelist {
        for entry in glob(&format!("{}{}", DIST, w))? {
            let path = entry?;
            if path.is_file() {
                output.push(Asset {
                    // If the directory isn't valid this wouldn't have worked.
                    path: String::from(path.to_str().unwrap()).replace("\\", "/"),
                });
            }
        }
    }

    Ok(output)
}

/// Write to asset.in which is a vec expression of static assets that will be
/// imported using the include!() macro for the `Config` struct
fn write_asset_file() -> Result<()> {
    let assets = acquire_assets()?;
    let asset_out = Path::new(&env::var("OUT_DIR").unwrap()).join("asset.in");

    let mut writer = BufWriter::new(File::create(asset_out)?);
    write!(writer, "vec![")?;
    for asset in assets {
        write!(writer, "{},", asset)?;
    }
    write!(writer, "]")?;

    Ok(())
}

/// Intermediary data structure used to hold the path of the file
pub struct Asset {
    pub path: String,
}

// This gives us the ToString impl as well. We're using this to write out what
// an asset would look like for the actual program into the asset.in file.
impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We know this will give the proper name so we can do this to get the
        // proper name for the asset without the rest of the beginning path
        let name = self.path
            .split(DIST)
            .skip(1)
            .collect::<Vec<&str>>()
            .pop()
            .unwrap();

        let asset =
            format!(
            r#"rustdoc::assets::Asset {{
                name: "{name}",
                contents: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/{path}")),
            }}"#,
            name = name,
            path = self.path,
        );

        write!(f, "{}", asset)
    }
}

/// Create source tests from the files in the `tests/source` directory.
fn generate_source_tests() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut generated_code = File::create(Path::new(&out_dir).join("source_generated.rs"))?;

    let source_dir = Path::new(SOURCE_TEST_DIR);
    for source_file in fs::read_dir(&source_dir)? {
        let source_file = source_file?.path();

        let test_name = Ident::new(source_file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| "Invalid source file stem")?);

        let source_file_path = source_file.to_str().unwrap();

        let test =
            quote! {
            #[test]
            fn #test_name() {
                use std::env;
                use tempdir::TempDir;

                let tempdir = TempDir::new("rustdoc-test").unwrap();
                let source_file = env::current_dir().unwrap().join(#source_file_path);
                let host = generate_analysis(&source_file, tempdir.path()).unwrap();
                if let Err(err) = check(&source_file, &host) {
                    println!("error: {}", err);

                    for err in err.iter().skip(1) {
                        println!("caused by: {}", err);
                    }

                    if let Some(backtrace) = err.backtrace() {
                        println!("backtrace: {:?}", backtrace);
                    }

                    panic!();
                }
            }
        };

        write!(generated_code, "{}", test.to_string())?;
    }

    Ok(())
}
