
use cargo::command_bridge::CommandBridge;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use error::*;

pub fn retrieve_metadata(manifest_path: &Path) -> CommandBridge {
    CommandBridge::new("cargo")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
}

pub fn generate_analysis(manifest_path: &PathBuf, is_verbose: bool) -> Result<CommandBridge> {

    check_manifest_path_points_to_cargo_toml(manifest_path)?;

    const RLS_TARGET: &str = "target/rls";
    let _target_dir = manifest_path
        .parent()
        .expect(
            "Unreachable. If not, fill a bug about `check_manifest_path_points_to_cargo_toml`",
        )
        .join(RLS_TARGET);

    let command = CommandBridge::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest_path)
        //.env("CARGO_TARGET_DIR", target_dir) // FIXME compiles
        .env("RUSTFLAGS", "-Z save-analysis")
        .stderr(Stdio::piped())
        .stdout(Stdio::null());

    let command = if is_verbose {
        command.arg("--verbose")
    } else {
        command
    };

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

fn check_manifest_path_points_to_cargo_toml(manifest_path: &PathBuf) -> Result<()> {

    const ERR_MSG: &str = "Expected manifest_path to point to Cargo.toml";

    if let Some(file_name) = manifest_path.file_name() {

        if file_name != "Cargo.toml" {
            bail!(ERR_MSG);
        }

    } else {
        bail!(ERR_MSG);
    }

    Ok(())
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
            let res = generate_analysis(&PathBuf::from("Cargo.toml"), false);
            // assert
            assert!(res.is_ok())
        }

        #[test]
        fn it_should_add_verbose_flag_when_verbosity_is_enabled() {
            // arrange
            // act
            let res = generate_analysis(&PathBuf::from("Cargo.toml"), true).unwrap();
            // assert
            assert!(res.args.contains(&OsString::from("--verbose")))
        }

        #[test]
        fn it_should_err_if_manifest_path_doesnt_point_to_cargo_toml() {
            // arrange
            let manifest_path = PathBuf::from("Hello.txt");
            // act
            let res = generate_analysis(&manifest_path, false);
            // assert
            assert!(res.is_err())
        }
    }

    mod check_manifest_path_points_to_cargo_toml {
        use super::*;

        #[test]
        fn it_exists() {
            // arrange
            let path = PathBuf::from("Cargo.toml");
            // act
            let res = check_manifest_path_points_to_cargo_toml(&path);
            // assert
            assert!(res.is_ok())
        }

        #[test]
        fn it_err_on_directories() {
            // arrange
            let path = PathBuf::from("/users/rusty/");
            // act
            let res = check_manifest_path_points_to_cargo_toml(&path);
            // assert
            assert!(res.is_err())
        }

        #[test]
        fn it_returns_err_if_filename_is_not_cargo_toml() {
            // arrange
            let path = PathBuf::from("/users/rusty/Rusty.toml");
            // act
            let res = check_manifest_path_points_to_cargo_toml(&path);
            // assert
            assert!(res.is_err())
        }
    }
}
