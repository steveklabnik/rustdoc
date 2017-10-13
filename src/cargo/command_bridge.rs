//! This struct is a simple bridge that we can inspect for test purpose

use std::collections::HashMap;
use std::convert::{AsRef, Into};
use std::default::Default;
use std::ffi::{OsStr, OsString};
use std::process::{Command, Stdio};

/// This temporary struct is used to inspect the generated command before calling the process::Command
/// builder, as we can't inspect its structure and make assertions on it
#[derive(Debug, Default)]
pub struct CommandBridge {
    pub command_name: String,
    pub args: Vec<OsString>,
    pub env: HashMap<OsString, OsString>,
    pub stdout: Option<Stdio>,
    pub stderr: Option<Stdio>,
}

impl CommandBridge {
    pub fn new(command_name: &str) -> Self {
        CommandBridge {
            command_name: String::from(command_name),
            ..Default::default()
        }
    }

    pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        let mut os_string = OsString::new();
        os_string.push(arg);
        self.args.push(os_string.to_os_string());
        self
    }

    pub fn env<S: AsRef<OsStr>>(mut self, key: S, value: S) -> Self {
        let mut key_str = OsString::new();
        let mut value_str = OsString::new();

        key_str.push(key.as_ref());
        value_str.push(value.as_ref());

        self.env.insert(key_str, value_str);

        self
    }

    pub fn stderr<S: Into<Stdio>>(mut self, stderr: S) -> Self {
        self.stderr = Some(stderr.into());
        self
    }

    pub fn stdout<S: Into<Stdio>>(mut self, stdout: S) -> Self {
        self.stdout = Some(stdout.into());
        self
    }

    pub fn to_command(self) -> Command {
        let mut command = Command::new(self.command_name);
        command.args(self.args);
        command.envs(self.env);

        if let Some(stdio) = self.stdout {
            command.stdout(stdio);
        }

        if let Some(stdio) = self.stderr {
            command.stderr(stdio);
        }

        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod arg {
        use super::*;

        #[test]
        fn it_exists() {
            // arrange
            let arg = OsString::from("some arg");
            // act
            let cmd = CommandBridge::new("some cmd").arg(&arg);
            // assert
            assert!(cmd.args.contains(&arg))
        }
    }

    mod env {
        use super::*;

        #[test]
        fn it_exists() {
            // arrange
            let key = OsString::from("hello");
            let value = OsString::from("world");
            // act
            let cmd = CommandBridge::new("plop").env(&key, &value);
            // assert
            assert_eq!(&value, cmd.env.get(&key).unwrap())
        }
    }

    mod stdout {
        use super::*;

        #[test]
        fn it_exists() {
            // arrange
            let stdout = Stdio::null();
            // act
            let _cmd = CommandBridge::new("plop").stdout(stdout);
            // assert
            // FIXME: is there is really no way to compare an Stdio instance ??
            //assert_eq!(stdout as *const (), cmd.stdout.unwrap() as *const ())
        }
    }

    mod stderr {
        use super::*;

        #[test]
        fn it_exists() {
            // arrange
            let stderr = Stdio::null();
            // act
            let _cmd = CommandBridge::new("plop").stderr(stderr);
            // assert
            // FIXME: is there is really no way to compare an Stdio instance ??
            //assert_eq!(stderr as *const (), cmd.stderr.unwrap() as *const ())
        }
    }
}
