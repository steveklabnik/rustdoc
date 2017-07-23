//! Source-based integration tests.
//!
//! This file contains a test that will run `rustdoc` on every file in the `tests/source`
//! directory, and compare the JSON output with comments in the source that indicate the expected
//! output.
//!
//! Supported syntax:
//!
//! ```ignore
//! // @has <JSON pointer> '<Regular expression>'
//! ```
//!
//! This comment requires that the JSON output from `rustdoc` contains a string that matches the
//! regular expression at the value pointed at by the JSON pointer. See [RFC6901] for JSON pointer
//! syntax.
//!
//! [RFC6901]: https://tools.ietf.org/html/rfc6901

extern crate rustdoc;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;

extern crate regex;
extern crate rls_analysis as analysis;
extern crate tempdir;

use std::fs::{self, File};
use std::io::{self, BufReader};
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;

use analysis::{AnalysisHost, Target};
use regex::Regex;
use serde_json::Value;
use tempdir::TempDir;

use rustdoc::DocData;

error_chain! {
    foreign_links {
        Analysis(analysis::AError);
        Io(io::Error);
        Json(serde_json::Error);
        Regex(regex::Error);
    }

    links {
        Rustdoc(rustdoc::error::Error, rustdoc::error::ErrorKind);
    }

    errors {
        JsonPointer(p: String) {
            description("JSON pointer matched no data")
            display("JSON pointer '{}' matched no data", p)
        }
        ValueMismatch(value: String, re: String) {
            description("The value did not match the regular expression")
            display("The value '{}' did not match the regular expression '{}'", value, re)
        }
        ValueType(value: Value) {
            description("The JSON pointer did not point at a string")
            display("The JSON pointer did not point at a string: {:?}", value)
        }
    }
}

/// Create analysis data from a given source file. Returns an analysis host with the data loaded.
fn generate_analysis(source_file: &Path, tempdir: &Path) -> Result<AnalysisHost> {
    let source_filename = source_file.to_str().unwrap();

    let rustc_status = Command::new("rustc")
        .args(&["-Z", "save-analysis"])
        .arg(source_filename)
        .current_dir(tempdir.to_str().unwrap())
        .status()?;
    if !rustc_status.success() {
        bail!("Compilation of {} failed", source_filename);
    }

    // rls-analysis expects the analysis files to be in a specific directory -- one usually created
    // by cargo.
    let save_analysis_dir = tempdir.join("save-analysis-temp");
    let expected_analysis_dir = tempdir
        .join("target")
        .join("rls")
        .join(&Target::Debug.to_string())
        .join("save-analysis");
    fs::create_dir_all(&expected_analysis_dir)?;
    fs::rename(&save_analysis_dir, &expected_analysis_dir)?;

    let host = AnalysisHost::new(Target::Debug);
    host.reload(&tempdir, &tempdir, true)?;

    Ok(host)
}

/// Runs all tests in a given source file.
fn check(source_file: &Path, host: &AnalysisHost) -> Result<()> {
    let package_name = source_file.file_stem().unwrap();
    let json = DocData::new(host, package_name.to_str().unwrap())?
        .to_json()?;
    let json = serde_json::from_str(&json)?;

    let source = BufReader::new(File::open(source_file)?);
    let mut found_test = false;
    for line in source.lines() {
        if let Some(test_case) = parse_test(&line?) {
            let (pointer, regex) = test_case?;
            run_test(&json, &pointer, &regex)?;
            if !found_test {
                found_test = true;
            }
        }
    }

    if !found_test {
        bail!("Found no tests in {}", source_file.display());
    }

    Ok(())
}

/// Optionally parses a test case from a single line. If the line contains a test case, returns a
/// Result containing a tuple of the JSON pointer and the regular expression. If there is no test
/// case contained in the line, returns `None`.
fn parse_test(line: &str) -> Option<Result<(String, Regex)>> {
    lazy_static! {
        static ref COMMAND_RE: Regex =
            Regex::new("@has (?P<pointer>[a-z/]+) '(?P<match>.+)'").unwrap();
    }

    if let Some(caps) = COMMAND_RE.captures(line) {
        let regex = match Regex::new(&caps["match"]) {
            Ok(regex) => regex,
            Err(err) => return Some(Err(err.into())),
        };
        let pointer = caps["pointer"].to_owned();
        Some(Ok((pointer, regex)))
    } else {
        None
    }
}

/// Compares a JSON pointer against a Regex.
fn run_test(json: &serde_json::Value, pointer: &str, regex: &Regex) -> Result<()> {
    let value = match json.pointer(pointer) {
        Some(value) => value,
        None => bail!(ErrorKind::JsonPointer(pointer.to_owned())),
    };

    let value = value.as_str().ok_or_else(
        || "The JSON pointer pointed at a type other than string",
    )?;

    if regex.is_match(&value) {
        Ok(())
    } else {
        bail!(ErrorKind::ValueMismatch(
            value.to_owned(),
            regex.as_str().to_owned(),
        ));
    }
}

/// The test that actually checks all the source tests.
///
/// Consider generating these tests in the build script to allow parallelism.
#[test]
fn source() {
    let source_dir = Path::new("tests/source");
    let tempdir = TempDir::new("rustdoc-test").unwrap();

    for source_file in fs::read_dir(source_dir).unwrap() {
        let source_file = source_file.unwrap();
        print!(
            "checking {} ... ",
            source_file.file_name().to_str().unwrap()
        );

        let source_file = std::env::current_dir().unwrap().join(source_file.path());
        let host = generate_analysis(&source_file, tempdir.path()).unwrap();
        check(&source_file, &host).unwrap();

        io::stdout().flush().unwrap();
        println!("ok");
    }
}

mod tests {
    use super::*;

    macro_rules! assert_err {
        ($err:expr, $kind:path) => {
            match *($err).kind() {
                $kind(..) => (),
                ref kind => panic!("unexpected error kind: {:?}", kind),
            }
        }
    }

    #[test]
    fn parse_test() {
        let (pointer, regex) = super::parse_test("// @has /test 'value'").unwrap().unwrap();
        assert_eq!(pointer, "/test");
        assert_eq!(regex.as_str(), "value");

        assert!(super::parse_test(r#"fn main() { println!("no test case"); }"#).is_none());

        let err = super::parse_test("// @has /test '['").unwrap().unwrap_err();
        assert_err!(err, ErrorKind::Regex);
    }

    #[test]
    fn run_test() {
        let json = json!({
            "test": "value",
        });

        assert!(super::run_test(&json, "/test", &Regex::new("value").unwrap()).is_ok());

        let err = super::run_test(&json, "/test", &Regex::new("wrong value").unwrap()).unwrap_err();
        assert_err!(err, ErrorKind::ValueMismatch);

        let err = super::run_test(&json, "/nonexistent", &Regex::new("value").unwrap())
            .unwrap_err();
        assert_err!(err, ErrorKind::JsonPointer);
    }
}
