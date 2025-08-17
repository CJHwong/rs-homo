use crate::gui::types::ThemeMode;
use crate::plugins::{Plugin, PluginContext, PluginResult};

/// Mermaid diagram rendering plugin
pub struct MermaidPlugin {
    initialized: bool,
}

impl MermaidPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
}

impl Plugin for MermaidPlugin {
    fn name(&self) -> &'static str {
        "mermaid"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn handles_language(&self, language: &str) -> bool {
        language == "mermaid"
    }

    fn process_code_block(
        &self,
        content: &str,
        language: &str,
        _context: &PluginContext,
    ) -> Option<PluginResult> {
        if !self.handles_language(language) {
            return None;
        }

        // Escape content for HTML display
        let html_escaped_content = content
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        // Escape content for HTML attribute
        let attr_escaped_raw = content
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");

        let html = format!(
            r#"<div class="mermaid-container" data-mermaid-source="{attr_escaped_raw}">
                <div class="mermaid-buttons">
                    <button class="mermaid-toggle-btn" onclick="toggleMermaidView(this)" title="Toggle rendered/raw view">View</button>
                    <button class="mermaid-copy-btn" onclick="copyMermaidCode(this)" title="Copy Mermaid source">Copy</button>
                </div>
                <div class="mermaid">{content}</div>
                <pre class="mermaid-raw" style="display: none;"><code>{html_escaped_content}</code></pre>
            </div>"#
        );

        Some(PluginResult {
            html,
            javascript: None, // JavaScript is provided globally
            css: None,        // CSS is provided globally
        })
    }

    fn get_javascript(&self, context: &PluginContext) -> Option<String> {
        let theme_config = match context.theme_mode {
            ThemeMode::Light => r#"
                theme: 'base',
                themeVariables: {
                    primaryColor: '#ff6b35',
                    primaryTextColor: '#24292f',
                    primaryBorderColor: '#d1d9e0',
                    lineColor: '#57606a',
                    secondaryColor: '#f6f8fa',
                    tertiaryColor: '#ffffff'
                }"#,
            ThemeMode::Dark => r#"
                theme: 'dark',
                themeVariables: {
                    primaryColor: '#ff6b35',
                    primaryTextColor: '#f0f6fc',
                    primaryBorderColor: '#30363d',
                    lineColor: '#8b949e',
                    secondaryColor: '#21262d',
                    tertiaryColor: '#161b22'
                }"#,
            ThemeMode::System => r#"
                theme: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'base',
                themeVariables: {
                    primaryColor: '#ff6b35',
                    primaryTextColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#f0f6fc' : '#24292f',
                    primaryBorderColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#30363d' : '#d1d9e0',
                    lineColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#8b949e' : '#57606a',
                    secondaryColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#21262d' : '#f6f8fa',
                    tertiaryColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#161b22' : '#ffffff'
                }"#,
        };

        let javascript = format!(
            r#"
// Mermaid Plugin JavaScript

// Initialize Mermaid when available
if (typeof mermaid !== 'undefined') {{
    mermaid.initialize({{
        startOnLoad: false,
        {theme_config}
    }});
    
    // Function to render Mermaid diagrams
    window.renderMermaidDiagrams = function() {{
        const mermaidElements = document.querySelectorAll('.mermaid');
        console.log('Found', mermaidElements.length, 'mermaid elements');
        
        mermaidElements.forEach(async (element, index) => {{
            const graphDefinition = element.textContent.trim();
            if (!graphDefinition) return;
            
            console.log('Rendering mermaid diagram', index, 'with content length:', graphDefinition.length);
            
            try {{
                element.innerHTML = '';
                const {{ svg }} = await mermaid.render(`mermaidChart${{Date.now()}}_${{index}}`, graphDefinition);
                element.innerHTML = svg;
                console.log('Successfully rendered diagram', index);
            }} catch (error) {{
                console.error('Mermaid rendering error for diagram', index, ':', error);
                element.innerHTML = '<div style="color: red; padding: 10px; font-family: monospace;">Mermaid rendering error: ' + error.message + '</div>';
            }}
        }});
    }};
    
    // Render diagrams after DOM is ready
    setTimeout(() => {{
        window.renderMermaidDiagrams();
    }}, 100);
    
    // Re-render diagrams when theme changes (for system theme)
    if (window.matchMedia) {{
        window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {{
            mermaid.initialize({{
                startOnLoad: false,
                theme: e.matches ? 'dark' : 'base',
                themeVariables: {{
                    primaryColor: '#ff6b35',
                    primaryTextColor: e.matches ? '#f0f6fc' : '#24292f',
                    primaryBorderColor: e.matches ? '#30363d' : '#d1d9e0',
                    lineColor: e.matches ? '#8b949e' : '#57606a',
                    secondaryColor: e.matches ? '#21262d' : '#f6f8fa',
                    tertiaryColor: e.matches ? '#161b22' : '#ffffff'
                }}
            }});
            
            window.renderMermaidDiagrams();
        }});
    }}
}}

// Copy function for Mermaid diagrams
window.copyMermaidCode = function(button) {{
    const container = button.closest('.mermaid-container');
    const rawSource = container.getAttribute('data-mermaid-source');
    const unescapedCode = rawSource
        .replace(/&amp;/g, '&')
        .replace(/&quot;/g, '"')
        .replace(/&#39;/g, "'");
    window.webkit.messageHandlers.copyText.postMessage(unescapedCode);
}};

// Toggle function for Mermaid rendered/raw view
window.toggleMermaidView = function(button) {{
    const container = button.closest('.mermaid-container');
    const renderedView = container.querySelector('.mermaid');
    const rawView = container.querySelector('.mermaid-raw');
    
    if (renderedView.style.display === 'none') {{
        renderedView.style.display = 'block';
        rawView.style.display = 'none';
        button.textContent = 'View';
        button.title = 'Toggle rendered/raw view';
    }} else {{
        renderedView.style.display = 'none';
        rawView.style.display = 'block';
        button.textContent = 'Raw';
        button.title = 'Toggle rendered/raw view';
    }}
}};

// Function to render new Mermaid diagrams in appended content
window.renderNewMermaidDiagrams = function(container) {{
    if (typeof mermaid === 'undefined') return;
    
    const newMermaidElements = container.querySelectorAll('.mermaid');
    newMermaidElements.forEach(async (element, index) => {{
        const graphDefinition = element.textContent.trim();
        if (!graphDefinition) return;
        
        try {{
            element.innerHTML = '';
            const {{ svg }} = await mermaid.render(`appendedChart${{Date.now()}}_${{index}}`, graphDefinition);
            element.innerHTML = svg;
        }} catch (error) {{
            console.error('Mermaid rendering error for appended content:', error);
            element.innerHTML = '<div style="color: red; padding: 10px;">Mermaid rendering error: ' + error.message + '</div>';
        }}
    }});
}};
"#
        );

        Some(javascript)
    }

    fn get_css(&self, _context: &PluginContext) -> Option<String> {
        let css = r#"
/* Mermaid Plugin Styles */
.mermaid-container {
    position: relative;
    margin: 16px 0;
}

.mermaid-buttons {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 10;
    display: flex;
    gap: 4px;
}

.mermaid-toggle-btn,
.mermaid-copy-btn {
    padding: 4px 8px;
    font-size: 12px;
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid #d0d7de;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-family-mono);
}

.mermaid-toggle-btn:hover,
.mermaid-copy-btn:hover {
    background: rgba(255, 255, 255, 1);
}

@media (prefers-color-scheme: dark) {
    .mermaid-toggle-btn,
    .mermaid-copy-btn {
        background: rgba(33, 38, 45, 0.9);
        border-color: #30363d;
        color: #f0f6fc;
    }
    
    .mermaid-toggle-btn:hover,
    .mermaid-copy-btn:hover {
        background: rgba(33, 38, 45, 1);
    }
}

.mermaid {
    background: var(--color-canvas-default);
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 16px;
    overflow: auto;
}

.mermaid-raw {
    margin: 0;
}

.mermaid-raw code {
    display: block;
    padding: 16px;
    background: var(--color-canvas-subtle);
    border-radius: 6px;
    overflow: auto;
    white-space: pre;
    font-family: var(--font-family-mono);
}
"#;

        Some(css.to_string())
    }

    fn get_external_scripts(&self) -> Vec<String> {
        vec!["https://cdn.jsdelivr.net/npm/mermaid@11.9.0/dist/mermaid.min.js".to_string()]
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing Mermaid plugin v{}", self.version());
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Shutting down Mermaid plugin");
        self.initialized = false;
        Ok(())
    }
}