error_chain! {
    errors {
        CrateErr(crate_name: &'static str) {
            description("Crate not found")
            display("Crate not found: \"{}\"", crate_name)
        }
    }

    foreign_links {
        Io(::std::io::Error);
        Serde(::serde_json::Error);
        Analysis(::analysis::AError);
    }
}
