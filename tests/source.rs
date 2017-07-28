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
extern crate jsonapi;

use std::fs::{self, File};
use std::io::{self, BufReader};
use std::io::prelude::*;
use std::path::{Path};
use std::process::Command;

use analysis::{AnalysisHost, Target};
use regex::Regex;
use serde_json::Value;
use rustdoc::{DocData};

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
        InvalidDirective(d: String) {
            description("Directive is not valid")
            display("Directive is not valid: {}", d)
        }
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

// If more directives are added, this could be converted into an enum.
#[derive(Debug)]
struct TestCase {
    pointer: String,
    regex: Regex,
    negated: bool,
}

/// Create analysis data from a given source file. Returns an analysis host with the data loaded.
fn generate_analysis(source_file: &Path, tempdir: &Path) -> Result<AnalysisHost> {
    let source_filename = source_file.to_str().ok_or_else(
        || "Source filename contained invalid UTF-8",
    )?;

    let rustc_status = Command::new("rustc")
        .args(&["-Z", "save-analysis"])
        .arg(source_filename)
        .current_dir(tempdir.to_str().expect(
            "tempdir filename contained invalid UTF-8",
        ))
        .status()?;
    if !rustc_status.success() {
        bail!("Compilation of {} failed", source_filename);
    }

    // rls-analysis expects the analysis files to be in a specific directory -- one usually created
    // by cargo.

    // this is a workaround for platform-specific behavior, see
    // https://doc.rust-lang.org/stable/std/fs/fn.rename.html#platform-specific-behavior
    #[cfg(not(windows))]
    fn workaround(tempdir: &Path) -> Result<()> {
        let save_analysis_dir = tempdir.join("save-analysis-temp");
        let expected_analysis_dir = tempdir
            .join("target")
            .join("rls")
            .join(&Target::Debug.to_string())
            .join("save-analysis");

        fs::create_dir_all(&expected_analysis_dir)?;
        fs::rename(&save_analysis_dir, &expected_analysis_dir)?;

        Ok(())
    }

    #[cfg(windows)]
    fn workaround(tempdir: &Path) -> Result<()> {
        let save_analysis_dir = tempdir.join("save-analysis-temp");
        let expected_analysis_dir = tempdir.join("target").join("rls").join(
            &Target::Debug.to_string(),
        );

        fs::create_dir_all(&expected_analysis_dir)?;

        let expected_analysis_dir = expected_analysis_dir.join("save-analysis");
        fs::rename(&save_analysis_dir, &expected_analysis_dir)?;

        Ok(())
    }

    workaround(&tempdir)?;

    let host = AnalysisHost::new(Target::Debug);
    host.reload(tempdir, tempdir, true)?;

    Ok(host)
}

/// Runs all tests in a given source file.
fn check(source_file: &Path, host: &AnalysisHost) -> Result<()> {
    let package_name = source_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| "Invalid source file stem")?;
    let json = DocData::new(host, package_name)?.to_json()?;
    let json = serde_json::from_str(&json)?;

    let source = BufReader::new(File::open(source_file)?);
    let mut found_test = false;
    for line in source.lines() {
        if let Some(test_case) = parse_test(&line?) {
            run_test(&json, test_case?)?;
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
fn parse_test(line: &str) -> Option<Result<TestCase>> {
    lazy_static! {
        static ref DIRECTIVE_RE: Regex =
            Regex::new(r"^[[:^alnum:]]*@(?P<directive>[a-z]+)").unwrap();
        static ref HAS_RE: Regex =
            Regex::new(r"@(?P<negated>!)?has (?P<pointer>[[:alnum:]/]+) '(?P<match>.+)'").unwrap();
    }

    if let Some(caps) = DIRECTIVE_RE.captures(line) {
        let directive = &caps["directive"];
        match directive {
            "has" => {
                if let Some(caps) = HAS_RE.captures(line) {
                    let regex = match Regex::new(&caps["match"]) {
                        Ok(regex) => regex,
                        Err(err) => return Some(Err(err.into())),
                    };
                    let pointer = caps["pointer"].to_owned();
                    return Some(Ok(TestCase {
                        pointer,
                        regex,
                        negated: caps.name("negated").is_some(),
                    }));
                }
            }
            _ => (),
        }

        let err = ErrorKind::InvalidDirective(directive.into());
        Some(Err(err.into()))
    } else {
        None
    }
}

/// Runs a test case. Returns `Ok(())` if the test passes, otherwise returns the reason the test
/// failed.
fn run_test(json: &serde_json::Value, case: TestCase) -> Result<()> {
    let value = match json.pointer(&case.pointer) {
        Some(value) => value,
        None => return Err(ErrorKind::JsonPointer(case.pointer).into()),
    };

    let value = value.as_str().ok_or(
        "The JSON pointer pointed at a type other than string",
    )?;

    if case.regex.is_match(value) == !case.negated {
        Ok(())
    } else {
        bail!(ErrorKind::ValueMismatch(
            value.to_owned(),
            case.regex.as_str().to_owned(),
        ));
    }
}

// Tests generated from the files in tests/source
include!(concat!(env!("OUT_DIR"), "/source_generated.rs"));

mod source_tests {
    #![cfg_attr(feature = "cargo-clippy", allow(trivial_regex))]

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
        let test = super::parse_test("// @has /test 'value'").unwrap().unwrap();
        assert_eq!(test.pointer, "/test");
        assert_eq!(test.regex.as_str(), "value");
        assert!(!test.negated);

        let test = super::parse_test("// @has /included/0/attributes/ 'a module'")
            .unwrap()
            .unwrap();
        assert_eq!(test.pointer, "/included/0/attributes/");
        assert_eq!(test.regex.as_str(), "a module");
        assert!(!test.negated);

        assert!(super::parse_test(r#"fn main() { println!("no test case"); }"#).is_none());

        let err = super::parse_test("// @has /test '['").unwrap().unwrap_err();
        assert_err!(err, ErrorKind::Regex);

        let err = super::parse_test("// @garbage /test 'abc'")
            .unwrap()
            .unwrap_err();
        assert_err!(err, ErrorKind::InvalidDirective);
    }

    #[test]
    fn run_test() {
        let json = json!({
            "test": "value",
        });

        super::run_test(
            &json,
            TestCase {
                pointer: "/test".into(),
                regex: Regex::new("value").unwrap(),
                negated: false,
            },
        ).unwrap();

        super::run_test(
            &json,
            TestCase {
                pointer: "/test".into(),
                regex: Regex::new("nonexistent").unwrap(),
                negated: true,
            },
        ).unwrap();

        let err = super::run_test(
            &json,
            TestCase {
                pointer: "/test".into(),
                regex: Regex::new("wrong value").unwrap(),
                negated: false,
            },
        ).unwrap_err();
        assert_err!(err, ErrorKind::ValueMismatch);

        let err = super::run_test(
            &json,
            TestCase {
                pointer: "/nonexistent".into(),
                regex: Regex::new("value").unwrap(),
                negated: false,
            },
        ).unwrap_err();
        assert_err!(err, ErrorKind::JsonPointer);
    }
}
