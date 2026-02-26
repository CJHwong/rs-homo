use ammonia::Builder;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use std::collections::{HashMap, HashSet};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::gui::types::ThemeMode;
use crate::plugins::{PluginContext, manager::PLUGIN_MANAGER};

/// Build an ammonia sanitizer that allows safe structural HTML tags
/// but strips dangerous elements like <script>, <iframe>, event handlers, etc.
fn html_sanitizer() -> Builder<'static> {
    let mut allowed_tags: HashSet<&str> = HashSet::new();
    // Standard safe inline/block tags
    for tag in &[
        "a",
        "abbr",
        "acronym",
        "b",
        "blockquote",
        "br",
        "cite",
        "code",
        "dd",
        "del",
        "details",
        "dfn",
        "div",
        "dl",
        "dt",
        "em",
        "figcaption",
        "figure",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "hr",
        "i",
        "img",
        "ins",
        "kbd",
        "li",
        "mark",
        "ol",
        "p",
        "picture",
        "pre",
        "q",
        "rp",
        "rt",
        "ruby",
        "s",
        "samp",
        "small",
        "source",
        "span",
        "strong",
        "sub",
        "summary",
        "sup",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "tr",
        "tt",
        "u",
        "ul",
        "var",
        "wbr",
    ] {
        allowed_tags.insert(tag);
    }

    let mut builder = Builder::new();
    builder
        .tags(allowed_tags)
        .add_tag_attributes("a", &["href", "title"])
        .add_tag_attributes("img", &["src", "alt", "title", "width", "height"])
        .add_tag_attributes("td", &["colspan", "rowspan", "align"])
        .add_tag_attributes("th", &["colspan", "rowspan", "align"])
        .add_tag_attributes("source", &["src", "type", "media"])
        .add_tag_attributes("details", &["open"])
        .url_schemes(HashSet::from(["http", "https", "mailto"]))
        .link_rel(Some("noopener noreferrer"))
        .strip_comments(true);
    builder
}

const LIGHT_THEME: &str = "InspiredGitHub";
const DARK_THEME: &str = "base16-ocean.dark";

/// Generate a URL-safe slug from heading text (GitHub-style).
/// Matches GitHub's algorithm: lowercase, drop everything except
/// letters/numbers/spaces/hyphens, then replace spaces with hyphens.
/// Consecutive hyphens are preserved (e.g. "A & B" → "a--b").
fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '-' {
            slug.extend(c.to_lowercase());
        } else if c == ' ' {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

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
        ThemeMode::Paper => LIGHT_THEME,
    };

    let theme = &ts.themes[theme_name];

    let parser = Parser::new_ext(markdown_input, options);
    let mut html_output = String::new();
    let mut code_block_text = String::new();
    let mut code_block_language = String::new();
    let mut in_code_block = false;

    // Heading state for generating anchor IDs
    let mut in_heading = false;
    let mut heading_level = HeadingLevel::H1;
    let mut heading_text = String::new(); // plain text for slug
    let mut heading_html = String::new(); // inner HTML content
    let mut slug_counts: HashMap<String, usize> = HashMap::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = level;
                heading_text.clear();
                heading_html.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let base_slug = slugify(&heading_text);
                let count = slug_counts.entry(base_slug.clone()).or_insert(0);
                let id = if *count == 0 {
                    base_slug
                } else {
                    format!("{base_slug}-{count}")
                };
                *count += 1;
                let n = heading_level_to_u8(heading_level);
                html_output.push_str(&format!("<h{n} id=\"{id}\">"));
                html_output.push_str(&heading_html);
                html_output.push_str(&format!("</h{n}>\n"));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(lang) = kind {
                    code_block_language = lang.to_string();
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // Try to process with plugin system first
                let context = PluginContext {
                    theme_mode: theme_mode.clone(),
                    is_streaming: false,
                    content_id: format!("block_{}", html_output.len()),
                };

                if let Some(plugin_result) = PLUGIN_MANAGER.process_code_block(
                    &code_block_text,
                    &code_block_language,
                    &context,
                ) {
                    // Plugin handled the code block
                    html_output.push_str(&plugin_result.html);
                } else {
                    // Fallback to standard syntax highlighting
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
                }

                code_block_text.clear();
                code_block_language.clear();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_block_text.push_str(&text);
                } else if in_heading {
                    heading_text.push_str(&text);
                    let mut temp = String::new();
                    html::push_html(&mut temp, std::iter::once(Event::Text(text)));
                    heading_html.push_str(&temp);
                } else {
                    let mut temp_html = String::new();
                    html::push_html(&mut temp_html, std::iter::once(Event::Text(text)));
                    html_output.push_str(&temp_html);
                }
            }
            Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
                let mut temp = String::new();
                html::push_html(&mut temp, std::iter::once(Event::Code(text)));
                heading_html.push_str(&temp);
            }
            Event::SoftBreak | Event::HardBreak if in_heading => {
                heading_text.push(' ');
                let mut temp = String::new();
                html::push_html(&mut temp, std::iter::once(event));
                heading_html.push_str(&temp);
            }
            Event::Html(raw_html) | Event::InlineHtml(raw_html) => {
                let sanitized = html_sanitizer().clean(&raw_html).to_string();
                if in_heading {
                    heading_html.push_str(&sanitized);
                } else {
                    html_output.push_str(&sanitized);
                }
            }
            e => {
                if in_heading {
                    let mut temp = String::new();
                    html::push_html(&mut temp, std::iter::once(e));
                    heading_html.push_str(&temp);
                } else {
                    let mut temp_html = String::new();
                    html::push_html(&mut temp_html, std::iter::once(e));
                    html_output.push_str(&temp_html);
                }
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
        ThemeMode::Paper => LIGHT_THEME,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Introduction"), "introduction");
    }

    #[test]
    fn test_slugify_special_characters_preserve_double_hyphens() {
        // "&" is dropped, surrounding spaces each become hyphens → double hyphen
        assert_eq!(
            slugify("Current Architecture & Pain Points"),
            "current-architecture--pain-points"
        );
        assert_eq!(
            slugify("2. Current Architecture & Pain Points"),
            "2-current-architecture--pain-points"
        );
    }

    #[test]
    fn test_slugify_apostrophe_dropped() {
        assert_eq!(
            slugify("Mapping GoFreight's Permission Model to KeyCloak"),
            "mapping-gofreights-permission-model-to-keycloak"
        );
    }

    #[test]
    fn test_slugify_numbered_headings() {
        assert_eq!(
            slugify("4. Mapping GoFreight's Permission Model to KeyCloak"),
            "4-mapping-gofreights-permission-model-to-keycloak"
        );
    }

    #[test]
    fn test_slugify_hyphens_preserved() {
        assert_eq!(slugify("pre-existing condition"), "pre-existing-condition");
    }

    #[test]
    fn test_slugify_trims_leading_trailing() {
        assert_eq!(slugify("  hello  "), "hello");
        assert_eq!(slugify("...hello..."), "hello");
    }

    #[test]
    fn test_slugify_mixed_special_chars() {
        // colons, parens, commas all dropped
        assert_eq!(
            slugify("Step 1: Install (Required)"),
            "step-1-install-required"
        );
    }

    #[test]
    fn test_heading_ids_in_parsed_html() {
        let html = parse_markdown("## Current Architecture & Pain Points");
        assert!(
            html.contains("id=\"current-architecture--pain-points\""),
            "Expected double hyphen in heading id, got: {html}"
        );
    }

    #[test]
    fn test_duplicate_heading_ids() {
        let html = parse_markdown("## Foo\n\n## Foo\n\n## Foo");
        assert!(html.contains("id=\"foo\""), "First heading should be 'foo'");
        assert!(
            html.contains("id=\"foo-1\""),
            "Second heading should be 'foo-1'"
        );
        assert!(
            html.contains("id=\"foo-2\""),
            "Third heading should be 'foo-2'"
        );
    }

    #[test]
    fn test_sanitize_script_tag() {
        let html = parse_markdown("<script>alert('xss')</script>");
        assert!(
            !html.contains("<script"),
            "Script tags must be stripped, got: {html}"
        );
    }

    #[test]
    fn test_sanitize_event_handler() {
        let html = parse_markdown(r#"<img src=x onerror="alert('xss')">"#);
        assert!(
            !html.contains("onerror"),
            "Event handlers must be stripped, got: {html}"
        );
    }

    #[test]
    fn test_sanitize_iframe() {
        let html = parse_markdown(r#"<iframe src="https://evil.com"></iframe>"#);
        assert!(
            !html.contains("<iframe"),
            "Iframe tags must be stripped, got: {html}"
        );
    }

    #[test]
    fn test_sanitize_preserves_safe_html() {
        let html = parse_markdown("<details><summary>Click</summary>\n\nContent\n\n</details>");
        assert!(
            html.contains("<details>"),
            "Details tag must be preserved, got: {html}"
        );
        assert!(
            html.contains("<summary>"),
            "Summary tag must be preserved, got: {html}"
        );
    }

    #[test]
    fn test_sanitize_preserves_inline_formatting() {
        let html = parse_markdown("x<sup>2</sup> and H<sub>2</sub>O and <mark>hi</mark>");
        assert!(
            html.contains("<sup>"),
            "Sup tag must be preserved, got: {html}"
        );
        assert!(
            html.contains("<sub>"),
            "Sub tag must be preserved, got: {html}"
        );
        assert!(
            html.contains("<mark>"),
            "Mark tag must be preserved, got: {html}"
        );
    }

    #[test]
    fn test_sanitize_strips_javascript_url() {
        let html = parse_markdown(r#"<a href="javascript:alert(1)">click</a>"#);
        assert!(
            !html.contains("javascript:"),
            "javascript: URLs must be stripped, got: {html}"
        );
    }
}
