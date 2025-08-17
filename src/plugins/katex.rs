use crate::gui::types::ThemeMode;
use crate::plugins::{Plugin, PluginContext, PluginResult};

/// LaTeX/Math rendering plugin using KaTeX
pub struct LatexPlugin {
    initialized: bool,
}

impl LatexPlugin {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
}

impl Plugin for LatexPlugin {
    fn name(&self) -> &'static str {
        "latex"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn handles_language(&self, language: &str) -> bool {
        matches!(language, "math" | "latex" | "tex")
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

        // Determine if this is display math (block) or inline math
        let is_display_math = language == "math" || content.trim().starts_with("\\begin") || content.contains("\\\\");
        let math_class = if is_display_math { "math-display" } else { "math-inline" };

        let html = format!(
            r#"<div class="latex-container" data-latex-source="{attr_escaped_raw}">
                <div class="latex-buttons">
                    <button class="latex-toggle-btn" onclick="toggleLatexView(this)" title="Toggle rendered/raw view">View</button>
                    <button class="latex-copy-btn" onclick="copyLatexCode(this)" title="Copy LaTeX source">Copy</button>
                </div>
                <div class="latex-math {math_class}" data-latex="{attr_escaped_raw}"></div>
                <pre class="latex-raw" style="display: none;"><code>{html_escaped_content}</code></pre>
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
                trust: (context) => ['\\htmlId', '\\href'].includes(context.command),
                strict: false,
                output: 'htmlAndMathml',
                displayMode: false,
                throwOnError: false,
                errorColor: '#cc0000',
                macros: {
                    "\\RR": "\\mathbb{R}",
                    "\\NN": "\\mathbb{N}",
                    "\\ZZ": "\\mathbb{Z}",
                    "\\QQ": "\\mathbb{Q}",
                    "\\CC": "\\mathbb{C}"
                }"#,
            ThemeMode::Dark => r#"
                trust: (context) => ['\\htmlId', '\\href'].includes(context.command),
                strict: false,
                output: 'htmlAndMathml',
                displayMode: false,
                throwOnError: false,
                errorColor: '#ff6b6b',
                macros: {
                    "\\RR": "\\mathbb{R}",
                    "\\NN": "\\mathbb{N}",
                    "\\ZZ": "\\mathbb{Z}",
                    "\\QQ": "\\mathbb{Q}",
                    "\\CC": "\\mathbb{C}"
                }"#,
            ThemeMode::System => r#"
                trust: (context) => ['\\htmlId', '\\href'].includes(context.command),
                strict: false,
                output: 'htmlAndMathml',
                displayMode: false,
                throwOnError: false,
                errorColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#ff6b6b' : '#cc0000',
                macros: {
                    "\\RR": "\\mathbb{R}",
                    "\\NN": "\\mathbb{N}",
                    "\\ZZ": "\\mathbb{Z}",
                    "\\QQ": "\\mathbb{Q}",
                    "\\CC": "\\mathbb{C}"
                }"#,
        };

        let javascript = format!(
            r#"
// LaTeX Plugin JavaScript

// Initialize KaTeX when available
if (typeof katex !== 'undefined') {{
    console.log('KaTeX is available, initializing LaTeX plugin');
    
    // Default KaTeX options
    window.katexOptions = {{
        {theme_config}
    }};
    
    // Function to render LaTeX expressions
    window.renderLatexExpressions = function() {{
        const latexElements = document.querySelectorAll('.latex-math');
        console.log('Found', latexElements.length, 'LaTeX elements');
        
        latexElements.forEach((element, index) => {{
            const latexCode = element.getAttribute('data-latex');
            if (!latexCode) return;
            
            console.log('Rendering LaTeX expression', index, 'with content length:', latexCode.length);
            
            try {{
                // Determine if this should be display math
                const isDisplayMath = element.classList.contains('math-display') || 
                                     latexCode.includes('\\\\') || 
                                     latexCode.trim().startsWith('\\begin');
                
                const options = {{
                    ...window.katexOptions,
                    displayMode: isDisplayMath
                }};
                
                // Clear the element and render
                element.innerHTML = '';
                katex.render(latexCode, element, options);
                console.log('Successfully rendered LaTeX expression', index);
            }} catch (error) {{
                console.error('KaTeX rendering error for expression', index, ':', error);
                element.innerHTML = '<div style="color: ' + window.katexOptions.errorColor + '; padding: 10px; font-family: monospace;">LaTeX rendering error: ' + error.message + '</div>';
            }}
        }});
    }};
    
    // Render expressions after DOM is ready
    setTimeout(() => {{
        window.renderLatexExpressions();
    }}, 100);
    
    // Re-render when theme changes (for system theme)
    if (window.matchMedia) {{
        window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {{
            window.katexOptions.errorColor = e.matches ? '#ff6b6b' : '#cc0000';
            window.renderLatexExpressions();
        }});
    }}
}} else {{
    console.warn('KaTeX is not available. LaTeX expressions will not be rendered.');
}}

// Copy function for LaTeX expressions
window.copyLatexCode = function(button) {{
    const container = button.closest('.latex-container');
    const rawSource = container.getAttribute('data-latex-source');
    const unescapedCode = rawSource
        .replace(/&amp;/g, '&')
        .replace(/&quot;/g, '"')
        .replace(/&#39;/g, "'");
    window.webkit.messageHandlers.copyText.postMessage(unescapedCode);
}};

// Toggle function for LaTeX rendered/raw view
window.toggleLatexView = function(button) {{
    const container = button.closest('.latex-container');
    const renderedView = container.querySelector('.latex-math');
    const rawView = container.querySelector('.latex-raw');
    
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

// Function to render new LaTeX expressions in appended content
window.renderNewLatexExpressions = function(container) {{
    if (typeof katex === 'undefined') return;
    
    const newLatexElements = container.querySelectorAll('.latex-math');
    newLatexElements.forEach((element, index) => {{
        const latexCode = element.getAttribute('data-latex');
        if (!latexCode) return;
        
        try {{
            const isDisplayMath = element.classList.contains('math-display') || 
                                 latexCode.includes('\\\\') || 
                                 latexCode.trim().startsWith('\\begin');
            
            const options = {{
                ...window.katexOptions,
                displayMode: isDisplayMath
            }};
            
            element.innerHTML = '';
            katex.render(latexCode, element, options);
        }} catch (error) {{
            console.error('KaTeX rendering error for appended content:', error);
            element.innerHTML = '<div style="color: ' + window.katexOptions.errorColor + '; padding: 10px;">LaTeX rendering error: ' + error.message + '</div>';
        }}
    }});
}};
"#
        );

        Some(javascript)
    }

    fn get_css(&self, _context: &PluginContext) -> Option<String> {
        let css = r#"
/* LaTeX Plugin Styles */
.latex-container {
    position: relative;
    margin: 16px 0;
}

.latex-buttons {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 10;
    display: flex;
    gap: 4px;
}

.latex-toggle-btn,
.latex-copy-btn {
    padding: 4px 8px;
    font-size: 12px;
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid #d0d7de;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-family-mono);
}

.latex-toggle-btn:hover,
.latex-copy-btn:hover {
    background: rgba(255, 255, 255, 1);
}

@media (prefers-color-scheme: dark) {
    .latex-toggle-btn,
    .latex-copy-btn {
        background: rgba(33, 38, 45, 0.9);
        border-color: #30363d;
        color: #f0f6fc;
    }
    
    .latex-toggle-btn:hover,
    .latex-copy-btn:hover {
        background: rgba(33, 38, 45, 1);
    }
}

.latex-math {
    background: var(--color-canvas-default);
    border: 1px solid var(--color-border-default);
    border-radius: 6px;
    padding: 16px;
    overflow: auto;
    min-height: 40px;
}

.latex-math.math-display {
    text-align: center;
    padding: 20px 16px;
}

.latex-math.math-inline {
    display: inline-block;
    padding: 8px 12px;
    margin: 0 4px;
    vertical-align: middle;
}

.latex-raw {
    margin: 0;
}

.latex-raw code {
    display: block;
    padding: 16px;
    background: var(--color-canvas-subtle);
    border-radius: 6px;
    overflow: auto;
    white-space: pre;
    font-family: var(--font-family-mono);
}

/* KaTeX dark theme support */
@media (prefers-color-scheme: dark) {
    .latex-math .katex {
        color: #f0f6fc;
    }
    
    .latex-math .katex .mord {
        color: #f0f6fc;
    }
    
    .latex-math .katex .mbin,
    .latex-math .katex .mrel {
        color: #79c0ff;
    }
    
    .latex-math .katex .mop {
        color: #d2a8ff;
    }
    
    .latex-math .katex .mopen,
    .latex-math .katex .mclose {
        color: #ffa657;
    }
}

/* Ensure KaTeX fonts render properly */
.latex-math .katex {
    font-size: 1.1em;
}

.latex-math.math-display .katex {
    font-size: 1.3em;
}
"#;

        Some(css.to_string())
    }

    fn get_external_scripts(&self) -> Vec<String> {
        vec![
            "https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.js".to_string(),
        ]
    }

    fn get_external_css(&self) -> Vec<String> {
        vec![
            "https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.css".to_string(),
        ]
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing LaTeX plugin v{}", self.version());
        self.initialized = true;
        Ok(())
    }

    #[allow(dead_code)]
    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Shutting down LaTeX plugin");
        self.initialized = false;
        Ok(())
    }
}