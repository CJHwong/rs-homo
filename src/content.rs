use serde::{Deserialize, Serialize};

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
}

impl DocumentContent {
    pub fn new(markdown: String, html: String, title: String, file_path: Option<String>) -> Self {
        Self {
            markdown,
            html,
            mode: ViewMode::default(),
            title,
            file_path,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ViewMode::Preview => ViewMode::Source,
            ViewMode::Source => ViewMode::Preview,
        };
    }
}
