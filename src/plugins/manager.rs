use std::collections::HashMap;
use std::sync::RwLock;

use crate::plugins::{Plugin, PluginContext, PluginResult};

/// Plugin manager that handles registration and execution of plugins
pub struct PluginManager {
    plugins: RwLock<Vec<Box<dyn Plugin>>>,
    language_map: RwLock<HashMap<String, usize>>, // Maps language to plugin index
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
            language_map: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin with the manager
    pub fn register_plugin(&self, mut plugin: Box<dyn Plugin>) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize the plugin
        plugin.initialize()?;

        let plugin_name = plugin.name();
        log::info!("Registering plugin: {plugin_name}");

        let mut plugins = self.plugins.write().map_err(|_| "Failed to acquire plugins write lock")?;

        let plugin_index = plugins.len();
        plugins.push(plugin);

        // Language mapping is done dynamically during processing

        log::info!("Successfully registered plugin: {plugin_name} at index {plugin_index}");
        Ok(())
    }

    /// Process a code block using the appropriate plugin
    pub fn process_code_block(
        &self,
        content: &str,
        language: &str,
        context: &PluginContext,
    ) -> Option<PluginResult> {
        let plugins = self.plugins.read().ok()?;
        
        // First check if we have a cached mapping
        if let Ok(language_map) = self.language_map.read() {
            if let Some(&plugin_index) = language_map.get(language) {
                if let Some(plugin) = plugins.get(plugin_index) {
                    return plugin.process_code_block(content, language, context);
                }
            }
        }

        // If no cached mapping, search through plugins
        for (index, plugin) in plugins.iter().enumerate() {
            if plugin.handles_language(language) {
                // Cache this mapping for future use
                if let Ok(mut language_map) = self.language_map.write() {
                    language_map.insert(language.to_string(), index);
                }
                return plugin.process_code_block(content, language, context);
            }
        }

        None
    }

    /// Get all JavaScript from registered plugins
    pub fn get_all_javascript(&self, context: &PluginContext) -> String {
        let plugins = match self.plugins.read() {
            Ok(plugins) => plugins,
            Err(_) => return String::new(),
        };

        let mut all_js = Vec::new();

        for plugin in plugins.iter() {
            if let Some(js) = plugin.get_javascript(context) {
                all_js.push(js);
            }
        }

        all_js.join("\n\n")
    }

    /// Get all CSS from registered plugins
    pub fn get_all_css(&self, context: &PluginContext) -> String {
        let plugins = match self.plugins.read() {
            Ok(plugins) => plugins,
            Err(_) => return String::new(),
        };

        let mut all_css = Vec::new();

        for plugin in plugins.iter() {
            if let Some(css) = plugin.get_css(context) {
                all_css.push(css);
            }
        }

        all_css.join("\n\n")
    }

    /// Get all external script URLs from registered plugins
    pub fn get_all_external_scripts(&self) -> Vec<String> {
        let plugins = match self.plugins.read() {
            Ok(plugins) => plugins,
            Err(_) => return Vec::new(),
        };

        let mut all_scripts = Vec::new();

        for plugin in plugins.iter() {
            all_scripts.extend(plugin.get_external_scripts());
        }

        // Remove duplicates
        all_scripts.sort();
        all_scripts.dedup();
        all_scripts
    }

    /// Get all external CSS URLs from registered plugins
    pub fn get_all_external_css(&self) -> Vec<String> {
        let plugins = match self.plugins.read() {
            Ok(plugins) => plugins,
            Err(_) => return Vec::new(),
        };

        let mut all_css = Vec::new();

        for plugin in plugins.iter() {
            all_css.extend(plugin.get_external_css());
        }

        // Remove duplicates
        all_css.sort();
        all_css.dedup();
        all_css
    }

    /// Get list of all registered plugins
    #[allow(dead_code)]
    pub fn list_plugins(&self) -> Vec<(String, String)> {
        let plugins = match self.plugins.read() {
            Ok(plugins) => plugins,
            Err(_) => return Vec::new(),
        };

        plugins
            .iter()
            .map(|plugin| (plugin.name().to_string(), plugin.version().to_string()))
            .collect()
    }

    /// Shutdown all plugins
    #[allow(dead_code)]
    pub fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut plugins = self.plugins.write().map_err(|_| "Failed to acquire plugins write lock")?;
        
        for plugin in plugins.iter_mut() {
            if let Err(e) = plugin.shutdown() {
                log::warn!("Error shutting down plugin {}: {}", plugin.name(), e);
            }
        }

        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    pub static ref PLUGIN_MANAGER: PluginManager = PluginManager::new();
}

/// Initialize the plugin system with default plugins
pub fn initialize_plugins() -> Result<(), Box<dyn std::error::Error>> {
    // Register the Mermaid plugin
    let mermaid_plugin = Box::new(crate::plugins::mermaid::MermaidPlugin::new());
    PLUGIN_MANAGER.register_plugin(mermaid_plugin)?;

    // Register the LaTeX plugin
    let latex_plugin = Box::new(crate::plugins::katex::LatexPlugin::new());
    PLUGIN_MANAGER.register_plugin(latex_plugin)?;

    log::info!("Plugin system initialized");
    Ok(())
}