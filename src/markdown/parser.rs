use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::gui::types::ThemeMode;

const LIGHT_THEME: &str = "InspiredGitHub";
const DARK_THEME: &str = "base16-ocean.dark";

/// Parses a string of Markdown text and converts it into an HTML string.
///
/// Enables GitHub-style extensions like tables, footnotes, strikethrough, and task lists.
pub fn parse_markdown(markdown_input: &str) -> String {
    parse_markdown_with_theme(markdown_input, &ThemeMode::System)
}

/// Parses a string of Markdown text and converts it into an HTML string with theme-aware syntax highlighting.
pub fn parse_markdown_with_theme(markdown_input: &str, theme_mode: &ThemeMode) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Choose theme based on mode
    let theme_name = match theme_mode {
        ThemeMode::Light => LIGHT_THEME,
        ThemeMode::Dark => DARK_THEME,
        ThemeMode::System => LIGHT_THEME, // Default to light for system mode
    };

    let theme = &ts.themes[theme_name];

    let parser = Parser::new_ext(markdown_input, options);
    let mut html_output = String::new();
    let mut code_block_text = String::new();
    let mut code_block_language = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(lang) = kind {
                    code_block_language = lang.to_string();
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                let syntax = ps
                    .find_syntax_by_token(&code_block_language)
                    .unwrap_or_else(|| ps.find_syntax_by_token("txt").unwrap());

                let mut h = HighlightLines::new(syntax, theme);
                let mut html = String::from("<pre><code>");
                for line in LinesWithEndings::from(&code_block_text) {
                    let ranges = h.highlight_line(line, &ps).unwrap();
                    let mut line_html = String::new();
                    for (style, text) in ranges {
                        let fg = style.foreground;
                        let color = format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b);
                        let escaped_text = text.replace('&', "&amp;").replace('<', "&lt;");
                        line_html.push_str(&format!(
                            "<span style=\"color:{color}\">{escaped_text}</span>"
                        ));
                    }
                    html.push_str(&line_html);
                }
                html.push_str("</code></pre>");
                html_output.push_str(&html);

                code_block_text.clear();
                code_block_language.clear();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_block_text.push_str(&text);
                } else {
                    let mut temp_html = String::new();
                    html::push_html(&mut temp_html, std::iter::once(Event::Text(text)));
                    html_output.push_str(&temp_html);
                }
            }
            e => {
                let mut temp_html = String::new();
                html::push_html(&mut temp_html, std::iter::once(e));
                html_output.push_str(&temp_html);
            }
        }
    }

    html_output
}

/// Highlights markdown syntax and returns it as HTML with theme-aware syntax highlighting.
pub fn highlight_markdown_with_theme(markdown_input: &str, theme_mode: &ThemeMode) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("md").unwrap();

    // Choose theme based on mode
    let theme_name = match theme_mode {
        ThemeMode::Light => LIGHT_THEME,
        ThemeMode::Dark => DARK_THEME,
        ThemeMode::System => LIGHT_THEME, // Default to light for system mode
    };

    let theme = &ts.themes[theme_name];
    let mut h = HighlightLines::new(syntax, theme);

    let mut html_output = String::new();
    html_output.push_str("<pre style=\"background-color: var(--pre-bg-color); padding: 16px; border-radius: 6px; overflow: auto; white-space: pre-wrap; word-wrap: break-word;\"><code>");

    for line in LinesWithEndings::from(markdown_input) {
        let ranges = h.highlight_line(line, &ps).unwrap();
        for (style, text) in ranges {
            let fg = style.foreground;
            let color = format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b);
            let escaped_text = text.replace('&', "&amp;").replace('<', "&lt;");
            html_output.push_str(&format!(
                "<span style=\"color:{color}\">{escaped_text}</span>"
            ));
        }
    }

    html_output.push_str("</code></pre>");
    html_output
}
