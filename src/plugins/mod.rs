use crate::gui::types::ThemeMode;

pub mod manager;
pub mod mermaid;
pub mod katex;

/// Context information passed to plugins during processing
#[derive(Clone)]
pub struct PluginContext {
    pub theme_mode: ThemeMode,
    #[allow(dead_code)]
    pub is_streaming: bool,
    #[allow(dead_code)]
    pub content_id: String,
}

/// Result of plugin processing
#[derive(Debug, Clone)]
pub struct PluginResult {
    pub html: String,
    #[allow(dead_code)]
    pub javascript: Option<String>,
    #[allow(dead_code)]
    pub css: Option<String>,
}

/// Plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Returns the name of the plugin
    fn name(&self) -> &'static str;

    /// Returns the version of the plugin
    fn version(&self) -> &'static str;

    /// Returns the languages/content types this plugin handles
    fn handles_language(&self, language: &str) -> bool;

    /// Process a code block and return the processed HTML
    fn process_code_block(
        &self,
        content: &str,
        language: &str,
        context: &PluginContext,
    ) -> Option<PluginResult>;

    /// Get JavaScript code that needs to be injected into the page
    fn get_javascript(&self, context: &PluginContext) -> Option<String>;

    /// Get CSS styles that need to be injected into the page
    fn get_css(&self, context: &PluginContext) -> Option<String>;

    /// Get external script URLs that need to be loaded
    fn get_external_scripts(&self) -> Vec<String>;

    /// Get external CSS URLs that need to be loaded
    fn get_external_css(&self) -> Vec<String> {
        Vec::new() // Default implementation returns empty vector
    }

    /// Called when the plugin is initialized
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Called when the plugin is shut down
    #[allow(dead_code)]
    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

/// Plugin type for categorizing plugins
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PluginType {
    /// Plugins that process specific code block languages
    CodeProcessor,
    /// Plugins that enhance content rendering
    Renderer,
    /// Plugins that provide interactive features
    Interactive,
}