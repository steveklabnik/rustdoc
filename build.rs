#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;
extern crate walkdir;

use walkdir::WalkDir;
use std::fmt;
use std::fs::File;
use std::io::{Write, BufWriter, stderr};
use std::process::exit;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        WalkDir(::walkdir::Error);
    }
}

// Location of the dist folder
const DIST: &str = "frontend/dist/";

// Where to write the Assets out to
const ASSETOUT: &str = "src/asset.in";

/// Find the assets and write out the corresponding files
pub fn main() {
    if let Err(ref e) = write_asset_file() {
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
