use std::cell::Cell;
use std::fmt::{self, Debug};

use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug)]
pub struct Ui;

impl Ui {
    pub fn start_task(&self, name: &str) -> Task {
        let spinner = ProgressBar::new_spinner();

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
        eprintln!("warning: {}", message);
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
