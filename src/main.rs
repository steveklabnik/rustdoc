#![warn(missing_docs)]

//! Code used to drive the creation of documentation for Rust Crates.

extern crate clap;
extern crate env_logger;

extern crate rustdoc;

use clap::{App, Arg, ArgMatches, SubCommand};

use rustdoc::{build, error, Config, Result, Verbosity};

use std::process;
use std::path::PathBuf;

static ALL_ARTIFACTS: &[&str] = &["frontend", "json"];
static DEFAULT_ARTIFACTS: &[&str] = &["frontend"];

fn run() -> Result<()> {
    env_logger::init().expect("could not initialize logger");

    let version = env!("CARGO_PKG_VERSION");

    let matches = App::new("rustdoc")
        .version(version)
        .author("Steve Klabnik <steve@steveklabnik.com>")
        .about("Generate web-based documentation from your Rust code.")
        .arg(
            Arg::with_name("manifest-path")
                .long("manifest-path")
                // remove the unwrap in Config::new if this default_value goes away
                .default_value("./Cargo.toml")
                .help("The path to the Cargo manifest of the project you are documenting."),
        )
        .arg(Arg::with_name("quiet").short("q").long("quiet").help(
            "No output printed to stdout",
        ))
        .arg(Arg::with_name("verbose").short("v").long("verbose").help(
            "Use verbose output",
        ))

        // Flags that may be unsupported soon. Unimplemented for now.
        .arg(Arg::with_name("markdown-css").long("markdown-css").takes_value(true).help(
            "CSS files to include via <link> in a rendered Markdown file",
        ))
        .arg(Arg::with_name("html-in-header").long("html-in-header").takes_value(true).help(
            "Files to include inline in the <head> section of a rendered Markdown file or \
            generated documentation",
        ))
        .arg(
            Arg::with_name("html-before-content")
                .long("html-before-content")
                .takes_value(true)
                .help("Files to include inline between <body> and the content of a rendered \
                      Markdown file or generated documentation",)
        )
        .arg(
            Arg::with_name("html-after-content")
                .long("html-after-content")
                .takes_value(true)
                .help("Files to include inline between the content and </body> of a rendered \
                      Markdown file or generated documentation",)
        )
        .arg(
            Arg::with_name("markdown-before-content")
                .long("markdown-before-content")
                .takes_value(true)
                .help("Files to include inline between <body> and the content of a rendered \
                      Markdown file or generated documentation",)
        )
        .arg(
            Arg::with_name("markdown-after-content")
                .long("markdown-after-content")
                .takes_value(true)
                .help("Files to include inline between the content and </body> of a rendered \
                      Markdown file or generated documentation",)
        )
        .arg(
            Arg::with_name("markdown-playground-url")
                .long("markdown-playground-url")
                .takes_value(true)
                .help("URL to send code snippets to",)
        )
        .arg(Arg::with_name("markdown-no-toc").long("markdown-no-toc").help(
            "Do not include table of contents",
        ))
        .arg(Arg::with_name("extend-css").short("e").long("extend-css").takes_value(true).help(
            "To add some CSS rules with a given file to generate doc with your own theme. However, \
            your theme might break if the rustdoc's generated HTML changes, so be careful!",
        ))

        // Unsupported flags
        .arg(
            Arg::with_name("input-format")
                .short("r")
                .long("input-format")
                .takes_value(true)
                .hidden(true)
        )
        .arg(
            Arg::with_name("output-format")
                .short("w")
                .long("output-format")
                .takes_value(true)
                .hidden(true)
        )
        .arg(Arg::with_name("plugin-path").long("plugin-path").takes_value(true).hidden(true))
        .arg(Arg::with_name("passes").long("passes").takes_value(true).hidden(true))
        .arg(Arg::with_name("plugins").long("plugins").takes_value(true).hidden(true))
        .arg(Arg::with_name("no-defaults").long("no-defaults").hidden(true))

        // Renamed flags
        .arg(Arg::with_name("output").short("o").long("output").takes_value(true).hidden(true))
        .arg(Arg::with_name("crate-name").long("crate-name").takes_value(true).hidden(true))
        .arg(Arg::with_name("library-path").takes_value(true).hidden(true))
        .arg(Arg::with_name("cfg").long("cfg").takes_value(true).hidden(true))
        .arg(Arg::with_name("extern").long("extern").takes_value(true).hidden(true))
        .arg(Arg::with_name("target").long("target").takes_value(true).hidden(true))
        .arg(Arg::with_name("sysroot").long("sysroot").takes_value(true).hidden(true))
        .arg(Arg::with_name("test").long("test").hidden(true))
        .arg(Arg::with_name("test-args").long("test-args").hidden(true))

        .subcommand(
            SubCommand::with_name("build")
                .about("generates documentation")
                .arg(
                    Arg::with_name("artifacts")
                        .long("emit")
                        .use_delimiter(true)
                        .takes_value(true)
                        .possible_values(ALL_ARTIFACTS)
                        .help("Build artifacts to produce. Defaults to just the frontend."),
                )
                .arg(Arg::with_name("open").long("open").help(
                    "Open the docs in a web browser after building.",
                ))
                .arg(Arg::with_name("output").short("o").long("output").takes_value(true).help(
                    "Path to place the output",
                ))
                .arg(Arg::with_name("crate-name").long("crate-name").takes_value(true).help(
                    "Specify the name of this crate",
                ))
                .arg(
                    Arg::with_name("library-path")
                        .short("L")
                        .long("library-path")
                        .takes_value(true)
                        .use_delimiter(true)
                        .help("A list of directories to add to crate search path")
                )
                .arg(Arg::with_name("cfg").long("cfg").takes_value(true).help(
                    "Pass a --cfg to rustc",
                ))
                .arg(Arg::with_name("extern").long("extern").takes_value(true).help(
                    "Pass an --extern to rustc",
                ))
                .arg(Arg::with_name("target").long("target").takes_value(true).help(
                    "Target triple to document",
                ))
                .arg(Arg::with_name("sysroot").long("sysroot").takes_value(true).help(
                    "Override the system root",
                )),
        )
        .subcommand(SubCommand::with_name("open").about(
            "opens the documentation in a web browser",
        ))
        .subcommand(
            SubCommand::with_name("test")
                .about("runs documentation tests in the current crate")
                .arg(Arg::with_name("test-args").long("test-args").help(
                        "Arguments to pass to the test runner",
                )),
        )
        .get_matches();

    check_unsupported_flags(&matches)?;
    check_moved_flags(&matches)?;
    check_unimplemented_flags(&matches);

    // unwrap is okay because we take a default value
    let manifest_path = PathBuf::from(&matches.value_of("manifest-path").unwrap());
    let verbosity = if matches.is_present("quiet") {
        Verbosity::Quiet
    } else if matches.is_present("verbose") {
        Verbosity::Verbose
    } else {
        Verbosity::Normal
    };
    let mut config = Config::new(verbosity, manifest_path)?;

    match matches.subcommand() {
        ("build", Some(matches)) => {
            // FIXME: Workaround for clap #1056. Use `.default_value()` once the issue is fixed.
            let artifacts: Vec<&str> = matches
                .values_of("artifacts")
                .map(|values| values.collect())
                .unwrap_or_else(|| DEFAULT_ARTIFACTS.iter().map(|&artifact| artifact).collect());

            if let Some(output_path) = matches.value_of("output") {
                config.set_output_path(PathBuf::from(output_path));
            }

            build(&config, &artifacts)?;
            if matches.is_present("open") {
                config.open_docs()?;
            }
        }
        ("open", _) => {
            // First build the docs if they are not yet built.
            if !config.output_path().exists() {
                build(&config, DEFAULT_ARTIFACTS)?;
            }
            config.open_docs()?;
        }
        ("test", _) => {
            build(&config, ALL_ARTIFACTS)?;
            rustdoc::test(&config)?;
        }
        // default is to build
        _ => build(&config, DEFAULT_ARTIFACTS)?,
    }
    Ok(())
}

fn check_moved_flags(matches: &ArgMatches) -> Result<()> {
    let build_msg = "Use in conjunction with the build subcommand.";
    let moved_flags = [
        ("output", build_msg),
        ("crate-name", build_msg),
        ("library-path", build_msg),
        ("cfg", build_msg),
        ("extern", build_msg),
        ("target", build_msg),
        ("sysroot", build_msg),
        ("test", "Use in conjunction with the test subcommand."),
    ];

    for &(flag, err_msg) in moved_flags.iter() {
        if matches.args.contains_key(flag) {
            return Err(error::MovedFlag {
                flag_name: flag.to_string(),
                msg: err_msg.to_string(),
            }.into());
        }
    }

    Ok(())
}

fn check_unsupported_flags(matches: &ArgMatches) -> Result<()> {
    let unsupported_flags = [
        "input-format",
        "output-format",
        "plugin-path",
        "passes",
        "plugins",
        "no-defaults",
    ];

    for flag in unsupported_flags.iter() {
        if matches.is_present(flag) {
            return Err(error::UnsupportedFlag {
                flag_name: flag.to_string(),
            }.into());
        }
    }

    Ok(())
}

fn check_unimplemented_flags(matches: &ArgMatches) {
    let unimplemented_flags = [
        "markdown-css",
        "html-in-header",
        "html-before-content",
        "html-after-content",
        "markdown-before-content",
        "markdown-after-content",
        "markdown-playground-url",
        "markdown-no-toc",
        "extend-css",
    ];

    let unimplemented_build_flags = [
        "crate-name",
        "library-path",
        "cfg",
        "extern",
        "target",
        "sysroot",
    ];

    for flag in unimplemented_flags.iter() {
        if matches.is_present(flag) {
            unimplemented!("Flag {} is not implemented", flag);
        }
    }

    match matches.subcommand() {
        ("build", Some(matches)) => for flag in unimplemented_build_flags.iter() {
            if matches.is_present(flag) {
                unimplemented!("Flag {} is not implemented", flag);
            }
        },
        _ => {} // do nothing
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);

        eprintln!("Caused by: {}", e.cause());

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        let backtrace = e.backtrace().to_string();
        if !backtrace.is_empty() {
            eprintln!("Backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}
