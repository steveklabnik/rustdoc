error_chain! {
    errors {
        CrateErr(crate_name: &'static str) {
            description("Crate not found")
            display("Crate not found: \"{}\"", crate_name)
        }

        Cargo(status: ::std::process::ExitStatus, stderr: String) {
            description("Cargo command failed to run")
            display("Cargo failed with status {}. stderr:\n{}", status, stderr)
        }

        Json(location: &'static str) {
            description("Unexpected JSON response")
            display("Unexpected JSON response from {}", location)
        }
    }

    foreign_links {
        Io(::std::io::Error);
        Serde(::serde_json::Error);
        Analysis(::analysis::AError);
    }
}
