//! Additional metadata processing for the documentation.

use pulldown_cmark::{Event, Parser, Tag};

/// Returns the first paragraph of markdown, with formatting intact.
pub fn summary(markdown: &str) -> &str {
    if let Some(index) = markdown.find("\n\n") {
        &markdown[..index]
    } else {
        markdown
    }
}

/// Given a raw block of markdown, this function extracts the first paragraph and strips it of any
/// formatting.
pub fn plain_summary(markdown: &str) -> String {
    let parser = Parser::new(markdown);

    let mut summary = String::new();

    for event in parser {
        match event {
            Event::Text(text) => summary.push_str(&text),
            Event::Start(Tag::Code) | Event::End(Tag::Code) => summary.push('`'),
            Event::End(Tag::Paragraph) | Event::End(Tag::Header(_)) => break,
            _ => (),
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    #[test]
    fn summary() {
        assert_eq!(super::summary("Summary\n\nDetails"), "Summary");

        assert_eq!(super::summary("Example with `code`"), "Example with `code`");

        assert_eq!(super::summary("# Heading"), "# Heading");
    }

    #[test]
    fn plain_summary() {
        assert_eq!(&super::plain_summary("Summary\n\nDetails"), "Summary");

        assert_eq!(&super::plain_summary("# Heading\n\nDetails"), "Heading");

        assert_eq!(
            &super::plain_summary("hello [Rust](https://www.rust-lang.org) :)"),
            "hello Rust :)"
        );

        assert_eq!(
            &super::plain_summary(r#"hello [Rust](https://www.rust-lang.org "Rust") :)"#),
            "hello Rust :)"
        );

        assert_eq!(
            &super::plain_summary("code `let x = i32;` ..."),
            "code `let x = i32;` ..."
        );

        assert_eq!(
            &super::plain_summary("type `Type<'static>` ..."),
            "type `Type<'static>` ..."
        );

        assert_eq!(&super::plain_summary("# top header"), "top header");

        assert_eq!(&super::plain_summary("## header"), "header");
    }
}
