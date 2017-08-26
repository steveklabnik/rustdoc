use std::cell::Cell;
use std::cmp;
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
            Verbosity::Normal => ProgressBar::new_spinner(),
            Verbosity::Quiet => ProgressBar::hidden(),
        };

        spinner.enable_steady_tick(50);
        spinner.set_style(ProgressStyle::default_spinner().template(
            "{spinner} {prefix}: {wide_msg}",
        ));

        spinner.set_prefix(name);

        Task {
            ui: self,
            spinner,
            is_error: Cell::new(false),
        }
    }

    pub fn warn(&self, message: &str) {
        if self.verbosity > Verbosity::Quiet {
            eprintln!("warning: {}", message);
        }
    }
}

/// The verbosity of the output displayed to the user.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No output.
    Quiet,

    /// Normal output, with spinners.
    Normal,
}

impl Default for Verbosity {
    fn default() -> Verbosity {
        Verbosity::Normal
    }
}

pub struct Task<'a> {
    ui: &'a Ui,
    spinner: ProgressBar,
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
        self.spinner.set_message(message);
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

        self.spinner.finish_with_message(message);
    }
}
