use serde::{Deserialize, Serialize};

use crate::gui::types::StylePreferences;
use crate::markdown;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ViewMode {
    #[default]
    Preview,
    Source,
}

#[derive(Debug, Clone)]
pub enum ContentUpdate {
    FullReplace(DocumentContent),
    Append { markdown: String, html: String }, // Both markdown and HTML chunks to append
}

#[derive(Debug, Clone)]
pub struct DocumentContent {
    pub markdown: String,
    pub html: String,
    pub mode: ViewMode,
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub file_path: Option<String>,
    pub style_preferences: StylePreferences,
}

impl DocumentContent {
    pub fn new(markdown: String, html: String, title: String, file_path: Option<String>) -> Self {
        Self {
            markdown,
            html,
            mode: ViewMode::default(),
            title,
            file_path,
            style_preferences: StylePreferences::default(),
        }
    }

    /// Regenerates the HTML content with the current theme
    pub fn regenerate_html(&mut self) {
        self.html =
            markdown::parse_markdown_with_theme(&self.markdown, &self.style_preferences.theme);
    }
}
