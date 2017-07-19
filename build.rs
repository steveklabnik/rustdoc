#[macro_use]
extern crate lazy_static;
extern crate walkdir;
use walkdir::{WalkDir, WalkDirIterator, DirEntry};
use std::fmt;
use std::fs::File;
use std::ffi::OsStr;
use std::io::{Write, BufWriter};

// Location of the dist folder
const DIST: &str = "frontend/dist/";

// Where to write the Assets out to
const ASSETOUT: &str = "src/asset.in";

// Ignore directories specified here
lazy_static! {
    static ref IGNORE: Vec<&'static OsStr> = {
        vec![
            OsStr::new("ember-fetch")
        ]
    };
}

/// Find the assets and write out the corresponding files
pub fn main() {
    write_asset_file(acquire_assets());
}

/// Start the recursion from the top level of the frontend's dist folder
fn acquire_assets() -> Vec<Asset> {
    let mut output: Vec<Asset> = Vec::new();

    let filter_entries = |e: &DirEntry| !IGNORE.contains(&e.file_name());

    for entry in WalkDir::new(DIST).into_iter().filter_entry(filter_entries) {
        let entry = entry.expect("Unable to read directory entry");
        if entry.metadata().expect("Couldn't get metadata").is_file() {
            output.push(Asset {
                // If the directory isn't valid this wouldn't have worked.
                path: String::from(entry.path().to_str().unwrap()).replace("\\", "/"),
            });
        }
    }

    output
}

/// Write to asset.in which is a vec expression of static assets that will be
/// imported using the include!() macro for the `Config` struct
fn write_asset_file(assets: Vec<Asset>) {
    let mut writer = BufWriter::new(File::create(ASSETOUT).expect("Unable to create file"));
    writer.write_all(b"vec![").unwrap();
    for i in assets {
        writer.write_fmt(format_args!("{},", i)).unwrap();
    }
    writer.write_all(b"]\n").unwrap();
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
