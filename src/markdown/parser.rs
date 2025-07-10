use pulldown_cmark::{Options, Parser, html};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

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

/// Highlights markdown syntax and returns it as HTML.
pub fn highlight_markdown(markdown_input: &str) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("md").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    let mut html_output = String::new();
    html_output.push_str("<pre style=\"background-color:#2b303b;color:#c0c5ce;padding:16px;border-radius:6px;overflow:auto;font-family:'SF Mono','Menlo','Monaco',monospace;font-size:14px;line-height:1.4;\">");

    for line in LinesWithEndings::from(markdown_input) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        for (style, text) in ranges {
            let fg = style.foreground;
            let color = format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b);
            html_output.push_str(&format!(
                "<span style=\"color:{}\">{}</span>",
                color,
                html_escape(text)
            ));
        }
    }

    html_output.push_str("</pre>");
    html_output
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
