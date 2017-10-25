//! Source-based integration tests.
//!
//! See the [source test README](source/README.md) for more information.

extern crate rustdoc;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;

extern crate itertools;
extern crate jmespath;
extern crate regex;
extern crate rls_analysis as analysis;
extern crate rls_data as analysis_data;
extern crate serde;
extern crate shlex;
extern crate tempdir;

use std::fs::{self, File};
use std::io::prelude::*;
use std::io;
use std::path::Path;
use std::process::Command;

use analysis::{AnalysisHost, Target};
use analysis_data::config::Config as AnalysisConfig;
use itertools::EitherOrBoth::{Both, Left, Right};
use itertools::Itertools;
use jmespath::{ToJmespath, Variable};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

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
        NullMatch(p: String) {
            description("JMESPath query matched no data")
            display("JMESPath query '{}' matched no data", p)
        }
        TypeError(expected: String, value: Value) {
            description("The JMESPath query returned an unexpected type")
            display("The JMESPath query returned an unexpected type: expected {}, got {}",
                    expected, value)
        }
        TestFailure(msg: String) {
            description("The test failed")
            display("The test failed: {}", msg)
        }
        UnexpectedPass(msg: String) {
            description("The test passed, but it shouldn't")
            display("The test passed, but it shouldn't: {}", msg)
        }
    }
}

#[derive(Debug)]
enum Directive {
    /// The query result is a string and matches a regex.
    Has(Regex),

    /// The query result is a JSON value that equals another JSON value.
    Matches(Value),
}

impl PartialEq for Directive {
    fn eq(&self, other: &Self) -> bool {
        use Directive::*;

        match (self, other) {
            (&Has(ref re), &Has(ref other_re)) => re.as_str() == other_re.as_str(),
            (&Matches(ref val), &Matches(ref other_val)) => val == other_val,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
struct TestCase {
    jmespath: jmespath::Expression<'static>,
    negated: bool,
    directive: Directive,
}

impl TestCase {
    /// Runs the test case. Returns `Ok(())` if the test passes, otherwise returns the reason the
    /// test failed.
    fn run(&self, json: &serde_json::Value) -> Result<()> {
        let expression = self.jmespath.search(json).chain_err(
            || "could not evaluate JMESPath",
        )?;

        match self.directive {
            Directive::Has(ref re) => {
                let value = match *expression {
                    Variable::String(ref value) => value,
                    Variable::Null => {
                        if self.negated {
                            return Ok(());
                        } else {
                            bail!(ErrorKind::NullMatch(self.jmespath.as_str().into()));
                        }
                    }
                    ref value => {
                        let value = Value::deserialize(value.clone()).expect(
                            "could not deserialize JMESPath variable",
                        );

                        bail!(ErrorKind::TypeError("string".into(), value));
                    }
                };

                if re.is_match(&value) && self.negated {
                    let message = format!("`{}` matched the regex `{}`", value, re.as_str());
                    bail!(ErrorKind::UnexpectedPass(message));
                } else if !re.is_match(&value) && !self.negated {
                    let message = format!("`{}` did not match the regex `{}`", value, re.as_str());
                    bail!(ErrorKind::TestFailure(message));
                } else {
                    Ok(())
                }
            }
            Directive::Matches(ref json) => {
                let json = json.to_jmespath();

                if expression == json && self.negated {
                    let message = format!(
                        "The JMESPath query result `{}` matched the JSON `{}`, but it shouldn't",
                        expression,
                        json
                    );
                    bail!(ErrorKind::UnexpectedPass(message));
                } else if expression != json && !self.negated {
                    let message = format!(
                        "query result `{}` does not equal JSON `{}`",
                        expression,
                        json
                    );
                    bail!(ErrorKind::TestFailure(message));
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Create analysis data from a given source file. Returns an analysis host with the data loaded.
fn generate_analysis(source_file: &Path, tempdir: &Path) -> Result<AnalysisHost> {
    let source_filename = source_file.to_str().ok_or_else(
        || "Source filename contained invalid UTF-8",
    )?;

    let analysis_config = AnalysisConfig {
        full_docs: true,
        pub_only: true,
        ..Default::default()
    };

    // FIXME: Use the rustdoc command once #155 is resolved.
    let rustc_status = Command::new("rustc")
        .env(
            "RUST_SAVE_ANALYSIS_CONFIG",
            serde_json::to_string(&analysis_config)?,
        )
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
    let data = rustdoc::create_documentation(host, package_name)?;
    let json = serde_json::to_value(&data)?;

    let mut source = String::new();
    File::open(source_file)?.read_to_string(&mut source)?;
    let mut found_test = false;
    for (original_line_number, line) in join_line_continuations(&source) {
        if let Some(test_case) = parse_test(&line) {
            let test_case = test_case.chain_err(|| {
                format!("could not parse test on line {}", original_line_number)
            })?;

            test_case.run(&json).chain_err(|| {
                format!("test failed on line {}", original_line_number)
            })?;

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

/// Finds all of the tests in a given source file, by concatenating tests that span multiple lines.
///
/// Returns tuples of the line number that the tests started on and the full source of the test.
fn join_line_continuations(contents: &str) -> Vec<(usize, String)> {
    let mut tests = vec![];

    let mut last_line: Option<String> = None;
    let mut first_line_number = None;
    let mut concatenated_line = String::new();

    let lines = contents
        .lines()
        .map(|line| line.trim_right_matches(|c| c == '\r' || c == '\n'))
        .enumerate();

    for (line_number, line) in lines {
        let line_number = line_number + 1;
        let mut line = if let Some(ref last_line) = last_line {

            // Strip the common prefix from the last line.
            line.chars()
                .zip_longest(last_line.chars())
                .skip_while(|pair| match *pair {
                    Both(a, b) => a == b,
                    _ => false,
                })
                .flat_map(|pair| match pair {
                    Left(c) => Some(c),
                    Both(c, _) => Some(c),
                    Right(_) => None,
                })
                .collect()
        } else {
            String::from(line)
        };

        if first_line_number.is_none() {
            first_line_number = Some(line_number);
        }

        if line.ends_with('\\') {
            line.pop();
            if last_line.is_none() {
                last_line = Some(line.clone());
            }
            concatenated_line.push_str(&line);
        } else {
            concatenated_line.push_str(&line);
            tests.push((first_line_number.unwrap(), concatenated_line));
            last_line = None;
            first_line_number = None;
            concatenated_line = String::new();
        }
    }

    if last_line.is_some() {
        panic!("Trailing backslash at the end of the file");
    }

    tests
}

/// Optionally parses a test case from a single line. If the line contains a test case, returns a
/// Result containing a tuple of the JSON pointer and the regular expression. If there is no test
/// case contained in the line, returns `None`.
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

    let jmespath = jmespath::compile(&args[0]).chain_err(
        || "invalid JMESPath syntax",
    )?;

    let directive = match directive {
        "has" => {
            ensure!(args.len() == 2, "Not enough arguments");

            let regex = Regex::new(&args[1]).chain_err(|| "invalid regex syntax")?;

            Directive::Has(regex)
        }
        "matches" => {
            ensure!(args.len() == 2, "not enough arguments");

            let value = serde_json::from_str(&args[1]).chain_err(
                || "invalid JSON syntax",
            )?;

            Directive::Matches(value)
        }
        _ => {
            bail!("Unknown directive");
        }
    };

    Ok(TestCase {
        jmespath,
        negated,
        directive,
    })
}

/// Tests generated from the files in tests/source
mod docs {
    include!(concat!(env!("OUT_DIR"), "/source_generated.rs"));
}

mod tests {
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
    fn join_line_continuations() {
        let tests = super::join_line_continuations("// @has /test 'test'");
        assert_eq!(tests, vec![(1, String::from("// @has /test 'test'"))]);

        let tests = super::join_line_continuations("// @has /test 'test'\n// @has /test2 'test2'");
        assert_eq!(
            tests,
            vec![
                (1, String::from("// @has /test 'test'")),
                (2, String::from("// @has /test2 'test2'")),
            ]
        );

        let tests = super::join_line_continuations("// @has /multiline \\\n// 'test'");
        assert_eq!(tests, vec![(1, String::from("// @has /multiline 'test'"))]);

        let tests = super::join_line_continuations("// @has /short \\\n//    'longer next line'");
        assert_eq!(
            tests,
            vec![(1, String::from("// @has /short    'longer next line'"))]
        );
    }

    #[test]
    fn parse_test() {
        assert!(super::parse_test(r#"fn main() { println!("no test case"); }"#).is_none());

        let test = super::parse_test("// @has test 'value'").unwrap().unwrap();
        assert_eq!(
            test,
            TestCase {
                jmespath: jmespath::compile("test").unwrap(),
                negated: false,
                directive: Directive::Has(Regex::new("value").unwrap()),
            }
        );

        let test = super::parse_test("// @has included[0].attributes 'a module'")
            .unwrap()
            .unwrap();
        assert_eq!(
            test,
            TestCase {
                jmespath: jmespath::compile("included[0].attributes").unwrap(),
                negated: false,
                directive: Directive::Has(Regex::new("a module").unwrap()),
            }
        );

        let test = super::parse_test("// @!has some 'value'").unwrap().unwrap();
        assert_eq!(
            test,
            TestCase {
                jmespath: jmespath::compile("some").unwrap(),
                negated: true,
                directive: Directive::Has(Regex::new("value").unwrap()),
            }
        );

        let test = super::parse_test(r#"// @matches some '{ "json": "value" }'"#)
            .unwrap()
            .unwrap();
        assert_eq!(
            test,
            TestCase {
                jmespath: jmespath::compile("some").unwrap(),
                negated: false,
                directive: Directive::Matches(json!({ "json": "value" })),
            }
        );

        let err = super::parse_test("// @has /test '['").unwrap().unwrap_err();
        assert_err!(err, ErrorKind::InvalidDirective);

        let err = super::parse_test("// @garbage /test 'abc'")
            .unwrap()
            .unwrap_err();
        assert_err!(err, ErrorKind::InvalidDirective);

        let err = super::parse_test("// @has 1234 'pointer'")
            .unwrap()
            .unwrap_err();
        assert_err!(err, ErrorKind::InvalidDirective);
    }

    #[test]
    fn run_test() {
        let json = json!({
            "test": "value",
            "nonString": ["non", "string"],
        });

        let test = TestCase {
            jmespath: jmespath::compile("test").unwrap(),
            directive: Directive::Has(Regex::new("value").unwrap()),
            negated: false,
        };
        test.run(&json).unwrap();

        let test = TestCase {
            jmespath: jmespath::compile("test").unwrap(),
            directive: Directive::Has(Regex::new("nonexistent").unwrap()),
            negated: true,
        };
        test.run(&json).unwrap();

        let test = TestCase {
            jmespath: jmespath::compile("nonexistent").unwrap(),
            directive: Directive::Has(Regex::new("value").unwrap()),
            negated: true,
        };
        test.run(&json).unwrap();

        let test = TestCase {
            jmespath: jmespath::compile("nonString").unwrap(),
            directive: Directive::Matches(json!(["non", "string"])),
            negated: false,
        };
        test.run(&json).unwrap();

        let test = TestCase {
            jmespath: jmespath::compile("test").unwrap(),
            directive: Directive::Has(Regex::new("value").unwrap()),
            negated: true,
        };
        assert_err!(test.run(&json).unwrap_err(), ErrorKind::UnexpectedPass);

        let test = TestCase {
            jmespath: jmespath::compile("test").unwrap(),
            directive: Directive::Has(Regex::new("wrong value").unwrap()),
            negated: false,
        };
        assert_err!(test.run(&json).unwrap_err(), ErrorKind::TestFailure);

        let test = TestCase {
            jmespath: jmespath::compile("nonexistent").unwrap(),
            directive: Directive::Has(Regex::new("value").unwrap()),
            negated: false,
        };
        assert_err!(test.run(&json).unwrap_err(), ErrorKind::NullMatch);

        let test = TestCase {
            jmespath: jmespath::compile("nonString").unwrap(),
            directive: Directive::Has(Regex::new("value").unwrap()),
            negated: false,
        };
        assert_err!(test.run(&json).unwrap_err(), ErrorKind::TypeError);
    }
}
