use ammonia::Builder;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

use super::image::embed_local_image;
use crate::gui::types::ThemeMode;
use crate::plugins::{PluginContext, manager::PLUGIN_MANAGER};

fn is_data_url(value: &str) -> bool {
    value
        .trim_start()
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
}

/// Embed local `<img src>` files, and keep `data:` URLs only where they
/// are an image. A `data:text/html` link is a script vector; an image is not.
fn filter_url_attribute<'u>(
    element: &str,
    attribute: &str,
    value: &'u str,
    base_dir: &Path,
) -> Option<Cow<'u, str>> {
    let is_img_src = element == "img" && attribute == "src";
    if is_img_src {
        if is_data_url(value) && !value.trim_start()[5..].starts_with("image/") {
            return None;
        }
        return Some(embed_local_image(value, base_dir).map_or(Cow::Borrowed(value), Cow::Owned));
    }
    if (attribute == "href" || attribute == "src") && is_data_url(value) {
        return None;
    }
    Some(Cow::Borrowed(value))
}

/// Build an ammonia sanitizer that allows safe structural HTML tags
/// but strips dangerous elements like <script>, <iframe>, event handlers, etc.
fn html_sanitizer(base_dir: &Path) -> Builder<'static> {
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
        .url_schemes(HashSet::from(["http", "https", "mailto", "data"]))
        .link_rel(Some("noopener noreferrer"))
        .strip_comments(true);
    let base_dir = base_dir.to_path_buf();
    builder.attribute_filter(move |element, attribute, value| {
        filter_url_attribute(element, attribute, value, &base_dir)
    });
    builder
}

const LIGHT_THEME: &str = "InspiredGitHub";
const DARK_THEME: &str = "base16-ocean.dark";

// Loading these deserializes every bundled grammar and theme, and a fresh
// set recompiles its regexes on first use. Streaming parses once per chunk,
// so the sets are loaded once per process.
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

fn theme_for(theme_mode: &ThemeMode) -> &'static Theme {
    let theme_name = match theme_mode {
        ThemeMode::Dark => DARK_THEME,
        ThemeMode::Light | ThemeMode::System | ThemeMode::Paper => LIGHT_THEME,
    };
    &THEME_SET.themes[theme_name]
}

/// Append `code` to `out` as syntax-colored spans.
fn push_highlighted(out: &mut String, code: &str, syntax: &SyntaxReference, theme: &Theme) {
    let mut highlighter = HighlightLines::new(syntax, theme);
    for line in LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, &SYNTAX_SET).unwrap();
        for (style, text) in ranges {
            let fg = style.foreground;
            let escaped_text = text.replace('&', "&amp;").replace('<', "&lt;");
            // Writing into a String cannot fail.
            let _ = write!(
                out,
                "<span style=\"color:#{:02x}{:02x}{:02x}\">{escaped_text}</span>",
                fg.r, fg.g, fg.b
            );
        }
    }
}

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
    parse_markdown_with_theme(markdown_input, &ThemeMode::System, Path::new("."))
}

/// Rewrite a markdown image to point at its embedded bytes, if it is local.
fn embed_image_event<'e>(event: Event<'e>, base_dir: &Path) -> Event<'e> {
    let Event::Start(Tag::Image {
        link_type,
        dest_url,
        title,
        id,
    }) = event
    else {
        return event;
    };
    let dest_url = embed_local_image(&dest_url, base_dir).map_or(dest_url, Into::into);
    Event::Start(Tag::Image {
        link_type,
        dest_url,
        title,
        id,
    })
}

/// Parses a string of Markdown text and converts it into an HTML string with theme-aware syntax highlighting.
/// `base_dir` is where relative image paths resolve; see `base_dir_for`.
pub fn parse_markdown_with_theme(
    markdown_input: &str,
    theme_mode: &ThemeMode,
    base_dir: &Path,
) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let theme = theme_for(theme_mode);
    // Built on the first raw HTML event, then reused for the whole document.
    let mut sanitizer: Option<Builder> = None;

    let parser =
        Parser::new_ext(markdown_input, options).map(|event| embed_image_event(event, base_dir));
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

    // pulldown-cmark writes the alt attribute from the text events between
    // Start(Image) and End(Image). Fed one event at a time it loses that
    // state, so an image is buffered and rendered in one call.
    let mut image_events: Option<Vec<Event>> = None;

    for event in parser {
        if let Some(events) = image_events.as_mut() {
            let is_end = matches!(event, Event::End(TagEnd::Image));
            events.push(event);
            if is_end {
                let target = if in_heading {
                    &mut heading_html
                } else {
                    &mut html_output
                };
                html::push_html(target, image_events.take().into_iter().flatten());
            }
            continue;
        }
        match event {
            Event::Start(Tag::Image { .. }) => {
                image_events = Some(vec![event]);
            }
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
                    let syntax = SYNTAX_SET
                        .find_syntax_by_token(&code_block_language)
                        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
                    html_output.push_str("<pre><code>");
                    push_highlighted(&mut html_output, &code_block_text, syntax, theme);
                    html_output.push_str("</code></pre>");
                }

                code_block_text.clear();
                code_block_language.clear();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_block_text.push_str(&text);
                } else if in_heading {
                    heading_text.push_str(&text);
                    html::push_html(&mut heading_html, std::iter::once(Event::Text(text)));
                } else {
                    html::push_html(&mut html_output, std::iter::once(Event::Text(text)));
                }
            }
            Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
                html::push_html(&mut heading_html, std::iter::once(Event::Code(text)));
            }
            Event::SoftBreak | Event::HardBreak if in_heading => {
                heading_text.push(' ');
                html::push_html(&mut heading_html, std::iter::once(event));
            }
            Event::Html(raw_html) | Event::InlineHtml(raw_html) => {
                let sanitized = sanitizer
                    .get_or_insert_with(|| html_sanitizer(base_dir))
                    .clean(&raw_html)
                    .to_string();
                if in_heading {
                    heading_html.push_str(&sanitized);
                } else {
                    html_output.push_str(&sanitized);
                }
            }
            e => {
                let target = if in_heading {
                    &mut heading_html
                } else {
                    &mut html_output
                };
                html::push_html(target, std::iter::once(e));
            }
        }
    }

    html_output
}

/// Highlights markdown syntax and returns it as HTML with theme-aware syntax highlighting.
pub fn highlight_markdown_with_theme(markdown_input: &str, theme_mode: &ThemeMode) -> String {
    let syntax = SYNTAX_SET.find_syntax_by_extension("md").unwrap();

    let mut html_output = String::new();
    html_output.push_str("<pre style=\"background-color: var(--pre-bg-color); padding: 16px; border-radius: 6px; overflow: auto; white-space: pre-wrap; word-wrap: break-word;\"><code>");

    push_highlighted(
        &mut html_output,
        markdown_input,
        syntax,
        theme_for(theme_mode),
    );
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

    /// A fixture directory holding one tiny PNG; unique per test so tests
    /// run in parallel without clobbering each other.
    fn image_fixture(test_name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("homo_{test_name}_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("img")).unwrap();
        std::fs::write(dir.join("img/dot.png"), b"\x89PNG").unwrap();
        dir
    }

    fn render_in(markdown: &str, dir: &std::path::Path) -> String {
        parse_markdown_with_theme(markdown, &ThemeMode::Light, dir)
    }

    #[test]
    fn test_markdown_image_relative_path_is_embedded() {
        let dir = image_fixture("md_relative");
        let html = render_in("![dot](img/dot.png)", &dir);
        assert!(
            html.contains(r#"src="data:image/png;base64,iVBORw==""#),
            "Relative image must be embedded, got: {html}"
        );
        assert!(
            html.contains(r#"alt="dot""#),
            "Alt text must stay, got: {html}"
        );
    }

    #[test]
    fn test_markdown_image_absolute_path_is_embedded() {
        let dir = image_fixture("md_absolute");
        let absolute = dir.join("img/dot.png");
        let html = render_in(&format!("![dot]({})", absolute.display()), Path::new("/"));
        assert!(
            html.contains("data:image/png;base64,"),
            "Absolute image must be embedded, got: {html}"
        );
    }

    #[test]
    fn test_markdown_image_missing_file_keeps_src_and_alt() {
        let dir = image_fixture("md_missing");
        let html = render_in("![gone](img/nope.png)", &dir);
        assert!(html.contains(r#"src="img/nope.png""#), "got: {html}");
        assert!(html.contains(r#"alt="gone""#), "got: {html}");
    }

    #[test]
    fn test_markdown_image_remote_url_passes_through() {
        let html = render_in("![r](https://example.com/a.png)", Path::new("."));
        assert!(
            html.contains(r#"src="https://example.com/a.png""#),
            "got: {html}"
        );
    }

    #[test]
    fn test_raw_html_image_relative_path_is_embedded() {
        let dir = image_fixture("raw_relative");
        let html = render_in(r#"<img src="img/dot.png" width="10">"#, &dir);
        assert!(
            html.contains("data:image/png;base64,iVBORw=="),
            "Raw <img> must be embedded, got: {html}"
        );
    }

    #[test]
    fn test_raw_html_image_data_uri_is_kept() {
        let html = render_in(r#"<img src="data:image/png;base64,AAAA">"#, Path::new("."));
        assert!(
            html.contains(r#"src="data:image/png;base64,AAAA""#),
            "Pasted data: image must survive sanitizing, got: {html}"
        );
    }

    #[test]
    fn test_sanitize_strips_data_url_outside_img() {
        let html = render_in(
            r#"<a href="data:text/html,<script>alert(1)</script>">x</a><img src="data:text/html,x">"#,
            Path::new("."),
        );
        assert!(
            !html.contains("data:"),
            "data: must only survive as an image, got: {html}"
        );
    }
}
