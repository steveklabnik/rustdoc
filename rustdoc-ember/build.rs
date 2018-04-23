//! This build script writes to `asset.in`, which is a `vec` expression of static assets that will
//! be embedded in the `rustdoc-ember` source with the `include!` macro.

#[macro_use]
extern crate quote;

extern crate glob;

use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process;

use glob::glob;
use quote::{ToTokens, Tokens};

/// Location of the ember frontend dist folder
const DIST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/dist/");

/// Intermediary data structure used to hold the path of the file
pub struct Asset {
    pub path: PathBuf,
}

fn run() -> io::Result<()> {
    let assets = acquire_assets().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let tokens = quote!(vec!#assets);

    let asset_out = Path::new(&env::var("OUT_DIR").unwrap()).join("asset.in");
    let mut file = File::create(asset_out)?;
    file.write_all(tokens.as_str().as_bytes())?;

    Ok(())
}

/// Start the recursion from the top level of the frontend's dist folder
fn acquire_assets() -> Result<Vec<Asset>, glob::GlobError> {
    let mut output: Vec<Asset> = Vec::new();
    let whitelist = vec![
        "assets/**/*",
        "crossdomain.xml",
        "ember-fetch/**/*",
        "index.html",
        "robots.txt",
    ];

    for w in whitelist {
        let pattern = format!("{}{}", DIST, w);
        for entry in glob(&pattern).unwrap() {
            let path = entry?;
            if path.is_file() {
                output.push(Asset { path });
            }
        }
    }

    Ok(output)
}

impl ToTokens for Asset {
    /// Turns an asset struct into a constructor expression.
    ///
    /// The inserted tokens should match `rustdoc-ember`'s definition of `Asset` exactly.
    fn to_tokens(&self, tokens: &mut Tokens) {
        let relative_path = self.path
            .strip_prefix(DIST)
            .ok()
            .and_then(|path| path.to_str())
            .unwrap();
        let absolute_path = self.path.to_str().unwrap();

        tokens.append(quote! {
            Asset {
                path: #relative_path,
                contents: include_bytes!(#absolute_path),
            }
        });
    }
}

fn main() {
    if let Err(ref e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
