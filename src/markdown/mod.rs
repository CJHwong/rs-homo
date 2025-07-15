//! Markdown module: provides parsing utilities for markdown to HTML.

mod parser;

pub use parser::{highlight_markdown_with_theme, parse_markdown, parse_markdown_with_theme};
