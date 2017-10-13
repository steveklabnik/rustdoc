
/// The kinds of targets that we can document.
#[derive(Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// A `bin` target.
    Binary,

    /// A `lib` target.
    Library,
}

/// A target of documentation.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// The kind of the target.
    pub kind: TargetKind,

    /// The name of the target.
    ///
    /// This is *not* the name of the target's crate, which is used to retrieve the analysis data.
    /// Use the [`crate_name`] method instead.
    ///
    /// [`crate_name`]: ./struct.Target.html#method.crate_name
    pub name: String,
}

impl Target {
    /// Returns the name of the target's crate.
    ///
    /// This name is equivalent to the target's name, with dashes replaced by underscores.
    pub fn crate_name(&self) -> String {
        self.name.replace('-', "_")
    }
}
