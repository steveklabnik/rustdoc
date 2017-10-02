
use cargo::command_bridge::CommandBridge;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use error::*;

pub fn retrieve_metadata(manifest_path: &Path) -> CommandBridge {
    let mut command = CommandBridge::new("cargo");
    command.arg("metadata")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1");
    command
}

pub fn generate_analysis(manifest_path: &PathBuf, is_verbose: bool) -> Result<CommandBridge> {

    let mut command = CommandBridge::new("cargo");

    //let target_dir = manifest_path
    //    .parent()
    //    .ok_or("Expected manifest_path to point to Cargo.toml")?
    //    .join("target/rls");

    command
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .env("RUSTFLAGS", "-Z save-analysis")
        //.env("CARGO_TARGET_DIR", target_dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::null());

    if is_verbose {
        command.arg("--verbose");
    }

    //match target.kind {
    //    TargetKind::Library => {
    //        command.arg("--lib");
    //    }
    //    TargetKind::Binary => {
    //      //  command.args(&["--bin", &target.name]);
    //        ()
    //    }
    //}

    Ok(command)
}


#[cfg(test)]
mod tests {
    pub use super::*;
    pub use std::ffi::OsString;

    mod retrieve_metadata {
        pub use super::*;

        #[test]
        fn it_exists() {
            // arrange
            let path = Path::new(".");
            // act
            retrieve_metadata(path);
            // assert
        }

        #[test]
        fn it_should_add_the_manifest_path_in_arguments() {
            // arrange
            let path = Path::new("my-manifest-path");
            // act
            let cmd = retrieve_metadata(path);
            // assert
            assert!(cmd.args.contains(&OsString::from("my-manifest-path")))
        }
    }

    mod generate_analysis {
        use super::*;

        #[test]
        fn it_exists() {
            // arrange
            // act
            let res = generate_analysis(&PathBuf::new(),false);
            // assert
            assert!(res.is_ok())
        }

        #[test]
        fn it_should_add_verbose_flag_when_verbosity_is_enabled() {
            // arrange
            // act
            let res = generate_analysis(&PathBuf::new(), true).unwrap();
            // assert
            assert!(res.args.contains(&OsString::from("--verbose")))
        }
    }
}