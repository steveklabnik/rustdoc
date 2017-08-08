//! Error types, traits and aliases.

error_chain! {
    errors {
        /// Thrown whenever a crate cannot be found
        CrateErr(crate_name: String) {
            description("Crate not found")
            display("Crate not found: \"{}\"", crate_name)
        }

        /// Thrown whenever Cargo fails to run properly when getting data for `rustdoc`
        Cargo(status: ::std::process::ExitStatus, stderr: String) {
            description("Cargo command failed to run")
            display("Cargo failed with status {}. stderr:\n{}", status, stderr)
        }

        /// Thrown whenever the `JSON` grabbed from somewhere else is not what is expected.
        /// This is usually thrown when grabbing data output from `Cargo`
        Json(location: String) {
            description("Unexpected JSON response")
            display("Unexpected JSON response from {}", location)
        }
    }

    foreign_links {
        Io(::std::io::Error)
            #[doc = "`std::io::Error` converted to an error-chain variant"];
        Serde(::serde_json::Error)
            #[doc = "`serde_json::Error` converted to an error-chain variant"];
        Analysis(::analysis::AError)
            #[doc = "`analysis::AError` converted to an error-chain variant"];
    }
}
