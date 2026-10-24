//! Markdown module: provides parsing utilities for markdown to HTML.

mod image;
mod parser;

pub use image::base_dir_for;

pub use parser::{highlight_markdown_with_theme, parse_markdown, parse_markdown_with_theme};
