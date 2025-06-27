use pulldown_cmark::{html, Options, Parser};

/// Parses a string of Markdown text and converts it into an HTML string.
///
/// Enables GitHub-style extensions like tables, footnotes, strikethrough, and task lists.
pub fn parse_markdown(markdown_input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown_input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
