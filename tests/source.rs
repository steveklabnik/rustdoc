//! Source-based integration tests.
//!
//! This file contains a test that will run `rustdoc` on every file in the `tests/source`
//! directory, and compare `rustdoc`'s JSON output with comments in the source that indicate the
//! expected output.
//!
//! Supported directives:
//!
//! ```ignore
//! // @has <JMESPath> '<Regular expression>'
//! ```
//!
//! This directive requires that a [JMESPath] query evaluated against the JSON output returns a
//! string that matches the regular expression.
//!
//! [JMESPath]: http://jmespath.org/

extern crate rustdoc;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;

extern crate jmespath;
extern crate jsonapi;
extern crate regex;
extern crate rls_analysis as analysis;
extern crate shlex;
extern crate tempdir;

use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::Path;
use std::process::Command;

use analysis::{AnalysisHost, Target};
use jmespath::{Expression, Variable};
use regex::Regex;

error_chain! {
    foreign_links {
        Analysis(analysis::AError);
        Io(io::Error);
        Jmespath(jmespath::JmespathError);
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
        NullValue(expr: String) {
            description("JMESPath matched no data")
            display("JMESPath '{}' matched no data", expr)
        }
        ValueMismatch(value: String, re: String) {
            description("The value did not match the regular expression")
            display("The value '{}' did not match the regular expression '{}'", value, re)
        }
        ValueType(json: String) {
            description("The JMESPath did not evaluate to a string")
            display("The JMESPath did not evaluate to a string: {}", json)
        }
    }
}

// If more directives are added, this could be converted into an enum.
#[derive(Debug)]
struct TestCase {
    jmespath: Expression<'static>,
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

    workaround(tempdir)?;

    let host = AnalysisHost::new(Target::Debug);
    host.reload(tempdir, tempdir)?;

    Ok(host)
}

/// Runs all tests in a given source file.
fn check(source_file: &Path, host: &AnalysisHost) -> Result<()> {
    let package_name = source_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| "Invalid source file stem")?;
    let data = rustdoc::create_json(host, package_name)?;

    let json = serde_json::from_str(&data)?;

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

/// Optionally parses a test case from a single line.
///
/// If the line contains a directive, returns `Some(Result<TestCase>)` that indicates whether
/// the parsing of the directive into a `TestCase` succeeded. If no directive is detected, returns
/// `None`.
fn parse_test(line: &str) -> Option<Result<TestCase>> {
    lazy_static! {
        static ref DIRECTIVE_RE: Regex =
            Regex::new(r"(?x)
                ^[[:^alnum:]]*@(?P<negated>!)?(?P<directive>[a-z]+)
                \s+
                (?P<args>.*)$
            ").unwrap();
    }

    if let Some(caps) = DIRECTIVE_RE.captures(line) {
        let directive = &caps["directive"];
        let result = parse_directive(directive, &caps["args"], caps.name("negated").is_some())
            .chain_err(|| ErrorKind::InvalidDirective(line.into()));
        Some(result)
    } else {
        None
    }
}

fn parse_directive(directive: &str, args: &str, negated: bool) -> Result<TestCase> {
    let args = match shlex::split(args) {
        Some(args) => args,
        None => bail!("Could not split directive arguments"),
    };

    match directive {
        "has" => {
            if args.len() != 2 {
                bail!("Not enough arguments");
            }

            let jmespath = jmespath::compile(&args[0]).chain_err(|| "invalid JMESPath")?;
            let regex = Regex::new(&args[1])?;

            Ok(TestCase {
                jmespath,
                regex,
                negated,
            })
        }
        _ => {
            bail!("Unknown directive");
        }
    }
}

/// Runs a test case. Returns `Ok(())` if the test passes, otherwise returns the reason the test
/// failed.
fn run_test(json: &serde_json::Value, case: TestCase) -> Result<()> {
    let variable = case.jmespath.search(json)?;

    let value = match *variable {
        Variable::String(ref s) => s,
        Variable::Null => bail!(ErrorKind::NullValue(case.jmespath.as_str().to_owned())),
        ref variable => bail!(ErrorKind::ValueType(variable.to_string())),
    };

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
        let test = super::parse_test("// @has test 'value'").unwrap().unwrap();
        assert_eq!(test.jmespath.as_str(), "test");
        assert_eq!(test.regex.as_str(), "value");
        assert!(!test.negated);

        let test = super::parse_test("// @has included[0].attributes 'a module'")
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "included[0].attributes");
        assert_eq!(test.regex.as_str(), "a module");
        assert!(!test.negated);

        let test = super::parse_test(
            r#"// @has 'some[?test == "value"].val | sort(@)' 'complex JMESPath'"#,
        ).unwrap()
            .unwrap();
        assert_eq!(
            test.jmespath.as_str(),
            r#"some[?test == "value"].val | sort(@)"#
        );
        assert_eq!(test.regex.as_str(), "complex JMESPath");
        assert!(!test.negated);

        assert!(super::parse_test(r#"fn main() { println!("no test case"); }"#).is_none());

        let err = super::parse_test("// @has test '['").unwrap().unwrap_err();
        assert_err!(err, ErrorKind::InvalidDirective);

        let err = super::parse_test("// @garbage test 'abc'")
            .unwrap()
            .unwrap_err();
        assert_err!(err, ErrorKind::InvalidDirective);

        let err = super::parse_test("// @has inval~~id] 'JMESPath'")
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
                jmespath: jmespath::compile("test").unwrap(),
                regex: Regex::new("value").unwrap(),
                negated: false,
            },
        ).unwrap();

        super::run_test(
            &json,
            TestCase {
                jmespath: jmespath::compile("test").unwrap(),
                regex: Regex::new("nonexistent").unwrap(),
                negated: true,
            },
        ).unwrap();

        let err = super::run_test(
            &json,
            TestCase {
                jmespath: jmespath::compile("test").unwrap(),
                regex: Regex::new("wrong value").unwrap(),
                negated: false,
            },
        ).unwrap_err();
        assert_err!(err, ErrorKind::ValueMismatch);

        let err = super::run_test(
            &json,
            TestCase {
                jmespath: jmespath::compile("nonexistent").unwrap(),
                regex: Regex::new("value").unwrap(),
                negated: false,
            },
        ).unwrap_err();
        assert_err!(err, ErrorKind::NullValue);
    }
}
