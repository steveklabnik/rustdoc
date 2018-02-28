//! Documentation tests.

use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::iter;
use std::path::{Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{Event, Parser, Tag};
use quote::{ToTokens, Tokens};
use serde_json::{self, Value};
use syn::{self, AttrStyle, Attribute, Block, Constness, Expr, ExprKind, FnDecl, FunctionRetTy,
          Generics, Ident, Item, ItemKind, MetaItem, PathParameters, PathSegment, Stmt, Unsafety,
          Visibility, WhereClause};

use Config;
use Result;
use json::{Document, Documentation};
use error;

struct Extern {
    /// name of the external crate
    name: String,

    /// Path to rlib
    location: PathBuf,
}

pub fn find_tests<'a>(docs: &'a Documentation) -> Vec<(&'a String, Vec<String>)> {
    let krate = docs.data.as_ref().unwrap();

    iter::once(krate)
        .chain(docs.included.iter().flat_map(|data| data))
        .map(|data| (&data.id, gather_tests(&data)))
        .collect()
}

/// Determine the dependencies and the location of the dependency artifacts
fn find_externs_for_crate(config: &Config) -> Result<Vec<Extern>> {
    let output = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&config.manifest_path)
        .args(&["--message-format", "json"])
        .output()?;
    if !output.status.success() {
        return Err(format_err!(
            "cargo did not exit successfully: {}",
            output.status
        ));
    }

    let output = String::from_utf8(output.stdout).expect("cargo did not output valid utf-8");

    let mut externs = vec![];

    for message in output.lines() {
        let message = serde_json::from_str::<Value>(message)?;
        let is_compiler_artifact = message
            .as_object()
            .unwrap()
            .get("reason")
            .and_then(Value::as_str)
            .map(|reason| reason == "compiler-artifact")
            .unwrap_or_default();

        if is_compiler_artifact {
            let name = message
                .pointer("/target/name")
                .and_then(|v| Some(v.to_string()))
                .unwrap();
            let location = message
                .pointer("/filenames/0")
                .and_then(|v| Some(PathBuf::from(v.as_str().unwrap())))
                .unwrap();

            externs.push(Extern { name, location });
        }
    }

    Ok(externs)
}

/// Find and prepare tests in the given document.
fn gather_tests(document: &Document) -> Vec<String> {
    if let Some(docs) = document.attributes.get("docs") {
        find_test_blocks(docs)
            .into_iter()
            .map(|block| {
                let crate_name = document.id.split("::").next().unwrap();
                preprocess(&block, crate_name)
            })
            .collect()
    } else {
        vec![]
    }
}

/// Returns the testable code blocks in a given markdown string.
///
/// Any formatting in the code blocks (`#`) will be removed.
fn find_test_blocks(docs: &str) -> Vec<String> {
    let mut tests = vec![];

    let mut parser = Parser::new(docs);

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(ref language))
                if language.is_empty() || language == "rust" =>
            {
                let mut test = String::new();
                while let Some(event) = parser.next() {
                    match event {
                        Event::End(Tag::CodeBlock(_)) => {
                            tests.push(test);
                            break;
                        }
                        Event::Text(ref line) => {
                            let line = line.trim();
                            let trimmed_line = if line.starts_with("##") {
                                &line[1..]
                            } else if line.starts_with("# ") {
                                &line[2..]
                            } else {
                                line
                            };
                            test.push_str(trimmed_line);
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    tests
}

/// Preprocess a test for later compilation and execution.
///
/// 1. First, inject the current crate as an `extern crate` if no `extern crate`s are present.
/// 2. Wrap the code in `fn main() {}` if there is no `main` function.
///
/// Any crate attributes are preserved at the top level.
fn preprocess(test: &str, crate_name: &str) -> String {
    if let Ok(mut ast) = syn::parse_crate(test) {
        // TODO if the extern crate has `#[macro_use]` we need to strip it out
        let has_extern_crate = ast.items.iter().any(|item| match item.node {
            ItemKind::ExternCrate(..) => true,
            _ => false,
        });

        let has_main_function = ast.items.iter().any(|item| match item.node {
            ItemKind::Fn(..) if item.ident == "main" => true,
            _ => false,
        });

        let mut stmts = ast.items
            .drain(..)
            .map(|item| Stmt::Item(Box::new(item)))
            .collect::<Vec<Stmt>>();

        if has_main_function {
            let main_fn_call = Stmt::Semi(Box::new(Expr {
                node: ExprKind::Call(
                    Box::new(Expr {
                        node: ExprKind::Path(
                            None,
                            PathSegment {
                                ident: Ident::new("main"),
                                parameters: PathParameters::none(),
                            }.into(),
                        ),
                        attrs: vec![],
                    }),
                    vec![],
                ),
                attrs: vec![],
            }));

            stmts.push(main_fn_call);
        }

        // TODO: Handle `#![doc(test(no_crate_inject))]`?
        if !has_extern_crate && crate_name != "std" {
            stmts.insert(
                0,
                Stmt::Item(Box::new(Item {
                    ident: Ident::new(crate_name),
                    vis: Visibility::Inherited,
                    attrs: vec![],
                    node: ItemKind::ExternCrate(None),
                })),
            )
        }

        let a_doc_test = ItemKind::Fn(
            Box::new(FnDecl {
                inputs: vec![],
                output: FunctionRetTy::Default,
                variadic: false,
            }),
            Unsafety::Normal,
            Constness::NotConst,
            None,
            Generics {
                lifetimes: vec![],
                ty_params: vec![],
                where_clause: WhereClause::none(),
            },
            Box::new(Block { stmts: stmts }),
        );

        let test_attr = Attribute {
            style: AttrStyle::Outer,
            value: MetaItem::Word(Ident::new("test")),
            is_sugared_doc: false,
        };

        ast.items.push(Item {
            ident: Ident::new("a_doc_test"),
            vis: Visibility::Inherited,
            attrs: vec![test_attr],
            node: a_doc_test,
        });

        let mut tokens = Tokens::new();
        ast.to_tokens(&mut tokens);
        let program = tokens.to_string();

        program
    } else {
        // If we couldn't parse the crate, then test compilation will fail anyways. Just wrap
        // everything in a test function.
        format!("#[test] fn a_doc_test() {{\n{}\n}}", test)
    }
}

pub fn save_tests(
    tests: &Vec<(&String, Vec<String>)>,
    save_path: &Path,
    crate_name: &str,
) -> Result<()> {
    DirBuilder::new().recursive(true).create(save_path)?;

    let mut mods = vec![];

    for &(ref id, ref tests) in tests {
        for (number, test) in tests.iter().enumerate() {
            // FIXME: Make the name based off the file and line number.
            let name = format!("{}_{}", id, number);

            // TODO make this a different function
            // filter test names into valid identifiers that can be put into `mod #ident`
            let name = name.replace("::", "_");
            //

            let filename = save_path.join(&name).with_extension("rs");
            let mut file = File::create(filename)?;
            file.write_all(test.as_bytes())?;

            mods.push(name);
        }
    }

    // TODO use syn here as well?
    let mut main = String::new();

    main.push_str(&format!("extern crate {};\n", crate_name));
    for m in mods {
        main.push_str(&format!("mod {};\n", m));
    }
    main.push_str("fn main() {}");
    let mut file = File::create(save_path.join("main.rs"))?;
    file.write_all(main.as_bytes())?;

    Ok(())
}

fn find_search_path(crate_externs: &Vec<Extern>) -> Result<PathBuf> {
    let e = crate_externs.first().ok_or::<::failure::Error>(
        error::DocTestErr {
            output: "No externs to get search path".to_string(),
        }.into(),
    )?;
    let path = e.location.parent().ok_or::<::failure::Error>(
        error::DocTestErr {
            output: "No parent for extern path".to_string(),
        }.into(),
    )?;
    Ok(path.to_path_buf())
}

pub fn compile_tests(config: &Config, save_path: &Path) -> Result<PathBuf> {
    static TEST_NAME: &str = "rustdoc-test";

    let crate_externs = find_externs_for_crate(config)?;

    let mut externs = vec![];
    for c in crate_externs.iter() {
        externs.push(format!("{}={}", &c.name, &c.location.to_string_lossy()));
    }

    let search_path = find_search_path(&crate_externs)?;

    let extern_args: Vec<_> = externs
        .into_iter()
        .flat_map(|arg| vec![String::from("--extern"), arg])
        .collect();

    let output = Command::new("rustc")
        .arg("main.rs")
        .arg("--test")
        .args(&["-o", TEST_NAME])
        .args(&["--cap-lints", "allow"])
        .arg("-L")
        .arg(search_path.to_str().unwrap())
        .args(extern_args)
        .current_dir(&save_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(error::DocTestErr { output: stderr }.into());
    }

    Ok(save_path.join(TEST_NAME))
}

pub fn execute_tests(binary: &Path) -> Result<()> {
    // spawn allows the test output to write to stdout so we are not waiting for all the tests to
    // complete before showing the user output
    let rustdoc_test = Command::new(binary).spawn()?;
    let output = rustdoc_test.wait_with_output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(error::DocTestErr { output: stderr }.into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn find_test_blocks() {
        let docs = indoc!(
            r#"
            Documentation example.

            ```
            assert!(true);
            ```

            A code example.

            ```c
            printf("Hello, world!\n");
            ```

            Another documentation example.

            ```rust
            assert_eq!(1 + 1, 2);
            ```
            "#
        );

        let tests = super::find_test_blocks(docs);
        assert_eq!(tests.len(), 2);

        let hidden_import = indoc!(
            r#"
            ```
            # use example::Example;
            ```
            "#
        );
        assert!(!super::find_test_blocks(hidden_import)[0].contains("#"));
    }

    #[test]
    fn preprocess() {
        assert!(!super::preprocess("not rust code", "some_crate").is_empty());

        assert_eq!(
            &super::preprocess("assert!(true);", "test_crate"),
            quote!{
                #[test]
                fn a_doc_test() {
                    extern crate test_crate;
                    assert!(true);
                }
            }.as_str()
        );

        assert_eq!(
            &super::preprocess(
                indoc!(
                    r#"
                        extern crate rustdoc;
                        use rustdoc::build;
                    "#
                ),
                "some_other_crate",
            ),
            quote!{
                #[test]
                fn a_doc_test() {
                    extern crate rustdoc;
                    use rustdoc::build;
                }
            }.as_str()
        );

        assert_eq!(
            &super::preprocess(
                indoc!(
                    r#"
                        fn main() { println!("Hello, world!"); }
                    "#
                ),
                "hello_world",
            ),
            quote!{
                #[test]
                fn a_doc_test() {
                    extern crate hello_world;
                    fn main() {
                        println!("Hello, world!");
                    }
                    main();
                }
            }.as_str()
        );
    }
}
