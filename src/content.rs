use serde::{Deserialize, Serialize};

use crate::gui::types::StylePreferences;
use crate::markdown;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ViewMode {
    #[default]
    Preview,
    Source,
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

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ViewMode::Preview => ViewMode::Source,
            ViewMode::Source => ViewMode::Preview,
        };
    }

    /// Regenerates the HTML content with the current theme
    pub fn regenerate_html(&mut self) {
        self.html =
            markdown::parse_markdown_with_theme(&self.markdown, &self.style_preferences.theme);
    }
}
