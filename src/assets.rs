//! Functions and data type declarations for copying frontend assets to the target directory.

use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::path::Path;

use error::*;

lazy_static! {
    /// The assets to be output in the target directory.
    pub static ref ASSETS: Vec<Asset> = include!(concat!(env!("OUT_DIR"), "/asset.in"));
}

/// Static assets compiled into the binary so we get a single executable. These are dynamically
/// generated with the build script based off of items in the `frontend/dist` folder.
pub struct Asset {
    /// Relative path of the file
    pub name: &'static str,

    /// Contents of the file
    pub contents: &'static str,
}

/// Output an asset file to a given directory
///
/// ## Arguments
///
/// - name: Name of the asset file
/// - path: Path to the directory to write the file out to
/// - data: Data to be written to the file
pub fn create_asset_file(name: &str, path: &Path, data: &str) -> Result<()> {
    let mut asset_path = path.to_path_buf();
    asset_path.push(name);

    // the name may contain one or more directories. we need to create them before trying to create
    // a file
    if let Some(parent) = asset_path.parent() {
        if parent != path {
            DirBuilder::new().recursive(true).create(parent)?;
        }
    }

    let mut file = File::create(asset_path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}
