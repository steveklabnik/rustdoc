//! Source-based integration tests.
//!
//! This file contains tests that will run `rustdoc` on every file in the `tests/source` directory,
//! and compare `rustdoc`'s JSON output with comments in the source that indicate the expected
//! output. A test is generated for every file in the `source` directory.
//!
//! These special comments, or directives, start with an `@` symbol and an identifier. They are
//! followed by a number of arguments. Each argument is space-separated, but arguments may contains
//! spaces if they are quoted by using single quotes. For convenience, an argument that is
//! surrounded by double quotes is considered quoted, but the double quotes *will* appear in the
//! output. Quotes inside the argument may be escaped with `\`.
//!
//! Supported directives:
//!
//! ```ignore
//! // @has <JMESPath> [<JSON expression>]
//! // @!has <JMESPath> [<JSON expression>]
//! ```
//!
//! ## @has
//!
//! This directive requires that a [`JMESPath`][JMESPath] query evaluated against the JSON output
//! matches a particular JSON expression. If the directive starts with a `!`, then the test passes
//! if the query result does _not_ match the expression.
//!
//! The JSON expression syntax uses double quotes to distinguish between strings and identifiers.
//!
//! ```ignore
//! true -> Value::Bool(true)
//! "true" -> Value::String(true)
//! { "value": "true" } -> Value::Object(..)
//! ```
//! For example, testing the result of a JMESPath expression against an object would look like this:
//!
//! ```no_run
//! // @has some.value '{ "docs": "Hello, world!" }``
//! ```
//!
//! If no expression argument is provided, then the argument is assumed to be `true`. This is
//! useful for calling functions:
//!
//! ```no_run
//! // @has 'data.attributes.docs | contains(@, `"the documentation"`)'
//! ```
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
extern crate regex;
extern crate rls_analysis as analysis;
extern crate tempdir;

use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::Path;
use std::process::Command;

use analysis::{AnalysisHost, Target};
use jmespath::{Expression, Variable};
use regex::Regex;
use serde_json::Value;

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
        ValueMismatch(result: String, value: String) {
            description("The query result did not match the value")
            display("The query result '{}' did not match the value '{}'", result, value)
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
    json: Value,
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

/// Splits an argument string into multiple arguments.
///
/// The splitting is a little quirky, because we want to allow writing unquoted JSON strings.
/// Individual arguments may be quoted using `'` or `"`. If an argument must contain a quote, it
/// may be escaped with `\`. Unescaped single quotes will be removed from the output, while double
/// quotes will not.
fn split_args(args: &str) -> Result<Vec<String>> {
    let mut current_arg = String::new();
    let mut quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let mut split_args = vec![];

    for c in args.chars() {
        match c {
            '\'' if !double_quoted => {
                if escaped {
                    current_arg.push(c);
                    escaped = false;
                } else {
                    if quoted {
                        split_args.push(current_arg);
                        current_arg = String::new();
                    }
                    quoted = !quoted;
                }
            }
            '\"' if !quoted => {
                escaped = false;
                double_quoted = !double_quoted;
                current_arg.push(c);
            }
            '\\' if escaped => {
                escaped = false;
                current_arg.push(c);
            }
            '\\' => escaped = true,
            ' ' if !quoted && !double_quoted => {
                if !current_arg.is_empty() {
                    split_args.push(current_arg);
                    current_arg = String::new();
                }
            }
            _ => current_arg.push(c),
        }
    }

    if quoted {
        bail!("Unclosed quote");
    }

    if escaped {
        bail!("Unclosed escape");
    }

    if !current_arg.is_empty() {
        split_args.push(current_arg);
    }

    Ok(split_args)
}

fn parse_directive(directive: &str, args: &str, negated: bool) -> Result<TestCase> {
    let args = split_args(args)?;

    match directive {
        "has" => {
            if args.len() < 1 {
                bail!("Not enough arguments");
            }

            let jmespath = jmespath::compile(&args[0]).chain_err(|| "invalid JMESPath")?;
            let json = args.get(1)
                .map(|json| serde_json::from_str(json))
                .unwrap_or_else(|| Ok(Value::Bool(true)))
                .chain_err(|| "invalid JSON")?;

            Ok(TestCase {
                jmespath,
                json,
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
fn run_test(rustdoc_output: &serde_json::Value, case: TestCase) -> Result<()> {
    let search_result = case.jmespath.search(rustdoc_output)?;

    if let Variable::Null = *search_result {
        bail!(ErrorKind::NullValue(case.jmespath.as_str().into()));
    }

    let json: Variable = case.json.into();
    if case.negated != (*search_result != json) {
        bail!(ErrorKind::ValueMismatch(
            search_result.to_string(),
            json.to_string(),
        ));
    }

    Ok(())
}

// Tests generated from the files in tests/source
include!(concat!(env!("OUT_DIR"), "/source_generated.rs"));

mod source_tests {
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
    fn split_args() {
        let args = super::split_args("hello").unwrap();
        assert_eq!(args, vec![String::from("hello")]);

        let args = super::split_args("'hello'").unwrap();
        assert_eq!(args, vec![String::from("hello")]);

        let args = super::split_args(r#""hello""#).unwrap();
        assert_eq!(args, vec![String::from(r#""hello""#)]);

        let args = super::split_args(r#"'"hello"'"#).unwrap();
        assert_eq!(args, vec![String::from(r#""hello""#)]);

        let args = super::split_args(r#""hello world""#).unwrap();
        assert_eq!(args, vec![String::from(r#""hello world""#)]);

        let args = super::split_args("one two three").unwrap();
        assert_eq!(
            args,
            vec![
                String::from("one"),
                String::from("two"),
                String::from("three"),
            ]
        );

        let args = super::split_args(r"back\\slash").unwrap();
        assert_eq!(args, vec![String::from(r"back\slash")]);

        let args = super::split_args("one 'two three'").unwrap();
        assert_eq!(args, vec![String::from("one"), String::from("two three")]);

        let args = super::split_args(r"one 'two\'s three'").unwrap();
        assert_eq!(args, vec![String::from("one"), String::from("two's three")]);

        super::split_args("one 'two").unwrap_err();

        super::split_args(r"one \").unwrap_err();

        assert_eq!(super::split_args("").unwrap(), Vec::<String>::new());
        assert_eq!(super::split_args(" ").unwrap(), Vec::<String>::new());

        assert_eq!(
            super::split_args("'' ''").unwrap(),
            vec![String::from(""), String::from("")]
        );
    }

    #[test]
    fn parse_test() {
        let test = super::parse_test(r#"// @has test "value""#)
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "test");
        assert_eq!(test.json, json!("value"));
        assert!(!test.negated);

        let test = super::parse_test(r#"// @!has test '"value"'"#)
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "test");
        assert_eq!(test.json, json!("value"));
        assert!(test.negated);

        let test = super::parse_test(r#"// @has 'test' "it\'s escaped""#)
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "test");
        assert_eq!(test.json, json!("it's escaped"));

        let test = super::parse_test(r#"// @has included[0].attributes "a module""#)
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "included[0].attributes");
        assert_eq!(test.json, json!("a module"));

        let test = super::parse_test(
            r#"// @has 'some[?test == "value"].val | sort(@)' "complex JMESPath""#,
        ).unwrap()
            .unwrap();
        assert_eq!(
            test.jmespath.as_str(),
            r#"some[?test == "value"].val | sort(@)"#
        );
        assert_eq!(test.json, json!("complex JMESPath"));

        let test = super::parse_test(r#"// @has 'assume.bool'"#)
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "assume.bool");
        assert_eq!(test.json, json!(true));

        let test = super::parse_test(r#"// @has complex '{ "complex": "json" }'"#)
            .unwrap()
            .unwrap();
        assert_eq!(test.jmespath.as_str(), "complex");
        assert_eq!(test.json, json!({ "complex": "json" }));

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
                json: json!("value"),
                negated: false,
            },
        ).unwrap();

        super::run_test(
            &json,
            TestCase {
                jmespath: jmespath::compile("test").unwrap(),
                json: json!("nonexistent"),
                negated: true,
            },
        ).unwrap();

        let err = super::run_test(
            &json,
            TestCase {
                jmespath: jmespath::compile("test").unwrap(),
                json: json!("wrong value"),
                negated: false,
            },
        ).unwrap_err();
        assert_err!(err, ErrorKind::ValueMismatch);

        let err = super::run_test(
            &json,
            TestCase {
                jmespath: jmespath::compile("nonexistent").unwrap(),
                json: json!("value"),
                negated: false,
            },
        ).unwrap_err();
        assert_err!(err, ErrorKind::NullValue);
    }
}
