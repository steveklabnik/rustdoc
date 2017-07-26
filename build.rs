#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate quote;
extern crate syn;
extern crate walkdir;

use std::env;
use std::fmt;
use std::fs::File;
use std::fs;
use std::io::{Write, BufWriter, stderr};
use std::path::Path;
use std::process::exit;
use std::thread;

use quote::Ident;
use walkdir::WalkDir;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        WalkDir(::walkdir::Error);
    }
}

const SOURCE_TEST_DIR: &str = "tests/source";

// Location of the dist folder
const DIST: &str = "frontend/dist/";

// Where to write the Assets out to
const ASSETOUT: &str = "src/asset.in";

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

    for entry in WalkDir::new(DIST).into_iter() {
        let entry = entry?;
        if entry.metadata()?.is_file() {
            output.push(Asset {
                // If the directory isn't valid this wouldn't have worked.
                path: String::from(entry.path().to_str().unwrap()).replace("\\", "/"),
            });
        }
    }

    Ok(output)
}

/// Write to asset.in which is a vec expression of static assets that will be
/// imported using the include!() macro for the `Config` struct
fn write_asset_file() -> Result<()> {
    let assets = acquire_assets()?;
    let mut writer = BufWriter::new(File::create(ASSETOUT)?);
    writer.write_all(b"vec![")?;
    for i in assets {
        writer.write_fmt(format_args!("{},", i))?;
    }
    writer.write_all(b"]\n")?;

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
        let mut buffer = String::from("\nAsset { \nname: \"");

        // We know this will give the proper name so we can do this to get the
        // proper name for the asset without the rest of the beginning path
        buffer.push_str(&self.path
            .split(DIST)
            .skip(1)
            .collect::<Vec<&str>>()
            .pop()
            .unwrap());

        buffer.push_str("\", \ncontents: include_str!(\"../");
        buffer.push_str(&self.path);
        buffer.push_str("\") \n}");
        write!(f, "{}", &buffer)
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
                check(&source_file, &host).unwrap();
            }
        };

        write!(generated_code, "{}", test.to_string())?;
    }

    Ok(())
}
