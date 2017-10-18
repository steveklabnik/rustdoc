//! Functions used to generate the documentation for Rust Crates.

#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[cfg_attr(test, macro_use)]
extern crate serde_json;
#[cfg_attr(test, macro_use)]
extern crate indoc;
#[cfg_attr(test, macro_use)]
extern crate quote;

extern crate indicatif;
extern crate open;
extern crate pulldown_cmark;
extern crate rls_analysis as analysis;
extern crate rls_data as analysis_data;
extern crate syn;
extern crate tempdir;

pub mod cargo;
pub mod error;
pub mod json;

mod test;
mod ui;

use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

use cargo::Target;
use error::*;
use json::Documentation;
use test::TestResult;
use ui::Ui;

pub use error::{Error, ErrorKind};
pub use json::create_documentation;
pub use ui::Verbosity;

/// A structure that contains various fields that hold data in order to generate doc output.
#[derive(Debug)]
pub struct Config {
    /// Interactions with the user interface.
    ui: Ui,

    /// Path to the `Cargo.toml` file for the crate being analyzed
    manifest_path: PathBuf,

    /// Contains the Cargo analysis output for the crate being documented
    host: analysis::AnalysisHost,
}

impl Config {
    /// Create a new `Config` based off the location of the manifest as well as assets generated
    /// during the build phase
    ///
    /// ## Arguments
    ///
    /// - `manifest_path`: The path to the `Cargo.toml` of the crate being documented
    pub fn new(verbosity: Verbosity, manifest_path: PathBuf) -> Result<Config> {
        let host = analysis::AnalysisHost::new(analysis::Target::Debug);

        if !manifest_path.is_file() || !manifest_path.ends_with("Cargo.toml") {
            bail!("The --manifest-path must be a path to a Cargo.toml file");
        }

        Ok(Config {
            ui: Ui::new(verbosity),
            manifest_path,
            host,
        })
    }

    /// Returns the directory containing the `Cargo.toml` of the crate being documented.
    pub fn root_path(&self) -> &Path {
        // unwrap() is safe, as manifest_path will point to a file
        self.manifest_path.parent().unwrap()
    }

    /// Returns the directory where output files should be placed
    pub fn output_path(&self) -> PathBuf {
        self.root_path().join("target").join("doc")
    }

    /// Returns the path to the generated documentation.
    pub fn documentation_path(&self) -> PathBuf {
        self.output_path().join("data.json")
    }

    /// Open the generated docs in a web browser.
    pub fn open_docs(&self) -> Result<()> {
        let mut index = self.output_path().join("index.html");

        // If we can't find the index at the root, try looking in the crate folder.
        if !index.is_file() {
            let metadata = cargo::retrieve_metadata(&self.manifest_path)?;
            let target = cargo::target_from_metadata(&self.ui, &metadata)?;
            index = self.output_path().join(target.crate_name()).join(
                "index.html",
            );
        }

        open::that(index)?;
        Ok(())
    }
}

/// Generate documentation for a crate. This can be tuned to output JSON and/or Web assets to view
/// documentation or use the JSON for other applications built on top of `rustdoc`.
///
/// ## Arguments
///
/// - `config`: The `Config` struct that contains the data needed to generate the documentation
/// - `artifacts`: A slice containing what assets should be output at the end
pub fn build(config: &Config, artifacts: &[&str]) -> Result<()> {
    let metadata = cargo::retrieve_metadata(&config.manifest_path)?;
    let target = cargo::target_from_metadata(&config.ui, &metadata)?;
    generate_and_load_analysis(config, &target)?;

    let output_path = config.output_path();
    fs::create_dir_all(&output_path)?;

    let json = {
        let task = config.ui.start_task("Generating JSON");
        task.report("In Progress");
        let docs = json::create_documentation(&config.host, &target.crate_name())?;
        serde_json::to_string(&docs)?
    };

    if artifacts.contains(&"json") {
        let json_path = output_path.join("data.json");
        let mut file = File::create(json_path)?;
        file.write_all(json.as_bytes())?;
    }

    // Now that we've generated the documentation JSON, we start the frontend as a subprocess to
    // generate the final output.
    if artifacts.contains(&"frontend") {
        let task = config.ui.start_task("Generating documentation");
        task.report("In Progress");

        let frontend = env::var("RUSTDOC_FRONTEND").unwrap_or_else(|_| {
            // If we're being run from cargo, use the binary in this repository. Otherwise, try to
            // find one in PATH.
            if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
                let profile = if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                };

                format!("{}/target/{}/rustdoc-ember", manifest_dir, profile)
            } else {
                String::from("rustdoc-ember")
            }
        });

        let mut frontend_proc = Command::new(&frontend)
            .arg("--output")
            .arg(config.output_path())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| if e.kind() == io::ErrorKind::NotFound {
                Error::with_chain(e, format!("frontend `{}` not found", frontend))
            } else {
                e.into()
            })?;

        {
            let stdin = frontend_proc.stdin.as_mut().unwrap();
            stdin.write_all(json.as_bytes())?;
        }

        let output = frontend_proc.wait_with_output()?;
        if !output.status.success() {
            task.error();
            drop(task);
            println!("\n{}", String::from_utf8_lossy(&output.stderr));
            bail!("frontend `{}` did not execute successfully", frontend);
        }
    }

    Ok(())
}

/// Run all documentation tests.
pub fn test(config: &Config) -> Result<()> {
    let doc_json = File::open(config.documentation_path()).chain_err(
        || "could not find generated documentation",
    )?;
    let docs: Documentation = serde_json::from_reader(doc_json)?;

    let krate = docs.data.as_ref().unwrap();
    let tests: Vec<_> = iter::once(krate)
        .chain(docs.included.iter().flat_map(|data| data))
        .map(|data| (&data.id, test::gather_tests(&data)))
        .collect();

    // Run the tests.
    static SUCCESS_MESSAGE: &str = "ok";
    static FAILURE_MESSAGE: &str = "FAILED";

    let num_tests: usize = tests.iter().map(|&(_, ref tests)| tests.len()).sum();
    println!("running {} tests", num_tests);

    let mut passed = 0;
    let mut failures = vec![];
    for (id, tests) in tests {
        for (number, test) in tests.iter().enumerate() {
            // FIXME: Make the name based off the file and line number.
            let name = format!("{} - {}", id, number);
            print!("test {} ... ", name);
            io::stdout().flush()?;

            let message = match test::run_test(config, test)? {
                TestResult::Success => {
                    passed += 1;
                    SUCCESS_MESSAGE
                }
                TestResult::Failure(output) => {
                    failures.push((name, output));
                    FAILURE_MESSAGE
                }
            };

            println!("{}", message);
        }
    }

    if !failures.is_empty() {
        // Print the output of each failure.
        for &(ref name, ref output) in &failures {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stdout = stdout.trim();

            if !stdout.is_empty() {
                println!("\n---- {} stdout ----\n{}", name, stdout);
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.trim();

            if !stderr.is_empty() {
                println!("\n---- {} stderr ----\n{}", name, stderr);
            }
        }

        // Print a summary of all failures at the bottom.
        println!("\nfailures:");
        for &(ref name, _) in &failures {
            println!("    {}", name);
        }
    }

    println!(
        "\ntest result: {}. {} passed; {} failed; 0 ignored; 0 measured; 0 filtered out",
        if failures.is_empty() {
            SUCCESS_MESSAGE
        } else {
            FAILURE_MESSAGE
        },
        passed,
        failures.len()
    );

    if !failures.is_empty() {
        process::exit(1);
    }

    Ok(())
}

/// Generate save analysis data of a crate to be used later by the RLS library later and load it
/// into the analysis host.
///
/// ## Arguments:
///
/// - `config`: Contains data for what needs to be output or used. In this case the path to the
///             `Cargo.toml` file
/// - `target`: The target to document
fn generate_and_load_analysis(config: &Config, target: &Target) -> Result<()> {
    let task = config.ui.start_task("Generating save analysis data");
    task.report("In progress");

    let analysis_result =
        cargo::generate_analysis(config, target, |progress| { task.report(progress); });

    if analysis_result.is_err() {
        task.error();
        return analysis_result;
    }

    drop(task);

    let task = config.ui.start_task("Loading save analysis data");
    task.report("In Progress");

    let root_path = config.root_path();
    config.host.reload(root_path, root_path)?;

    drop(task);

    Ok(())
}
