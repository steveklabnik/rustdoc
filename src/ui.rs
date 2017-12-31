use std::cell::Cell;
use std::fmt::{self, Debug};

use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Default)]
pub struct Ui {
    verbosity: Verbosity,
}

impl Ui {
    pub fn new(verbosity: Verbosity) -> Ui {
        Ui { verbosity }
    }

    pub fn start_task(&self, name: &str) -> Task {
        let spinner = match self.verbosity {
            Verbosity::Normal => Some(ProgressBar::new_spinner()),
            Verbosity::Quiet => Some(ProgressBar::hidden()),
            Verbosity::Verbose => None,
        };

        if let Some(ref spinner) = spinner {
            spinner.enable_steady_tick(50);
            spinner.set_style(
                ProgressStyle::default_spinner().template("{spinner} {prefix}: {wide_msg}"),
            );

            spinner.set_prefix(name);
        }

        Task {
            ui: self,
            name: name.to_owned(),
            spinner,
            is_error: Cell::new(false),
        }
    }

    pub fn warn(&self, message: &str) {
        if self.verbosity > Verbosity::Quiet {
            eprintln!("warning: {}", message);
        }
    }

    pub fn verbosity(&self) -> &Verbosity {
        &self.verbosity
    }
}

/// The verbosity of the output displayed to the user.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No output.
    Quiet,

    /// Normal output, with spinners.
    Normal,

    /// Verbose output. No spinners are displayed, and all intermediate output is printed.
    Verbose,
}

impl Default for Verbosity {
    fn default() -> Verbosity {
        Verbosity::Normal
    }
}

pub struct Task<'a> {
    ui: &'a Ui,
    name: String,
    spinner: Option<ProgressBar>,
    is_error: Cell<bool>,
}

impl<'a> Debug for Task<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("Task")
            .field("ui", &self.ui)
            .field("spinner", &String::from("Spinner {}"))
            .field("is_error", &self.is_error)
            .finish()
    }
}

impl<'a> Task<'a> {
    pub fn report(&self, message: &str) {
        if let Some(ref spinner) = self.spinner {
            spinner.set_message(message);
        } else {
            println!("{}: {}", self.name, message);
        }
    }

    pub fn report_verbose(&self, message: &str) {
        if self.ui.verbosity > Verbosity::Normal {
            println!("{}: {}", self.name, message);
        }
    }

    pub fn error(&self) {
        self.is_error.set(true);
    }
}

impl<'a> Drop for Task<'a> {
    fn drop(&mut self) {
        let message = if self.is_error.take() {
            "Error"
        } else {
            "Done"
        };

        if let Some(ref spinner) = self.spinner {
            spinner.finish_with_message(message);
        } else {
            println!("{}: {}", self.name, message);
        }
    }
}
