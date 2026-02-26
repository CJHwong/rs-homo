use crate::gui::types::ThemeMode;
use crate::plugins::{Plugin, PluginContext, PluginResult};

/// Mermaid diagram rendering plugin
pub struct MermaidPlugin {
    initialized: bool,
}

impl MermaidPlugin {
    pub fn new() -> Self {
        Self { initialized: false }
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
                    <button class="mermaid-copy-btn" onclick="copyMermaidImage(this)" title="Copy diagram as image">Copy Image</button>
                </div>
                <div class="mermaid">{html_escaped_content}</div>
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
            ThemeMode::Light => {
                r#"
                theme: 'base',
                themeVariables: {
                    primaryColor: '#ff6b35',
                    primaryTextColor: '#24292f',
                    primaryBorderColor: '#d1d9e0',
                    lineColor: '#57606a',
                    secondaryColor: '#f6f8fa',
                    tertiaryColor: '#ffffff'
                }"#
            }
            ThemeMode::Dark => {
                r#"
                theme: 'dark',
                themeVariables: {
                    primaryColor: '#ff6b35',
                    primaryTextColor: '#f0f6fc',
                    primaryBorderColor: '#30363d',
                    lineColor: '#8b949e',
                    secondaryColor: '#21262d',
                    tertiaryColor: '#161b22'
                }"#
            }
            ThemeMode::System => {
                r#"
                theme: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'base',
                themeVariables: {
                    primaryColor: '#ff6b35',
                    primaryTextColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#f0f6fc' : '#24292f',
                    primaryBorderColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#30363d' : '#d1d9e0',
                    lineColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#8b949e' : '#57606a',
                    secondaryColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#21262d' : '#f6f8fa',
                    tertiaryColor: window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? '#161b22' : '#ffffff'
                }"#
            }
            ThemeMode::Paper => {
                r#"
                theme: 'base',
                themeVariables: {
                    primaryColor: '#ddd8ce',
                    primaryTextColor: '#33302a',
                    primaryBorderColor: '#ddd8ce',
                    lineColor: '#7a7568',
                    secondaryColor: '#f3f1ea',
                    tertiaryColor: '#faf9f5'
                }"#
            }
        };

        let javascript = format!(
            r#"
// Mermaid Plugin JavaScript
//
// Rich Text Copy Flow
// --------------------
// Mermaid diagrams are rendered as SVGs, which most paste targets (Notes,
// Google Docs, etc.) cannot display. To make Cmd+A → Cmd+C produce a
// fully portable rich document we use a pre-cache + intercept strategy:
//
// Step 1 — CACHE: After each diagram renders (renderMermaidDiagrams /
//   renderNewMermaidDiagrams), cacheMermaidPng() converts the SVG to a
//   retina (2x) PNG via an off-screen canvas and stores the data URL on
//   the container element as a `data-png-src` attribute.  The 200ms
//   setTimeout ensures the SVG has been laid out so getBoundingClientRect
//   returns correct dimensions.
//
// Step 2 — INTERCEPT: A document-level `copy` event listener fires
//   synchronously when the user presses Cmd+C.  It clones the selected
//   range, checks for any `.mermaid-container` nodes, and replaces each
//   one with an <img> whose src is the cached PNG data URL.  The modified
//   HTML is written to e.clipboardData as text/html (plus the original
//   plain text as text/plain), then e.preventDefault() stops the default
//   copy.  If no Mermaid containers are in the selection the handler
//   returns early and WKWebView's native rich-text copy runs untouched.
//
// Step 3 — SINGLE DIAGRAM: The "Copy Image" button calls copyMermaidImage()
//   which performs the same SVG→canvas→PNG conversion on demand and sends
//   the base64 data to the native `copyImage` message handler, which
//   writes it to NSPasteboard as public.png (see src/gui/view.rs).

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
                // Step 1: pre-cache PNG for the copy event listener (see flow above)
                const mc = element.closest('.mermaid-container');
                if (mc) setTimeout(() => window.cacheMermaidPng(mc), 200);
            }} catch (error) {{
                console.error('Mermaid rendering error for diagram', index, ':', error);
                element.textContent = '';
                var errDiv = document.createElement('div');
                errDiv.style.cssText = 'color: red; padding: 10px; font-family: monospace;';
                errDiv.textContent = 'Mermaid rendering error: ' + error.message;
                element.appendChild(errDiv);
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

// Copy source code for Mermaid diagrams
window.copyMermaidCode = function(button) {{
    const container = button.closest('.mermaid-container');
    const rawSource = container.getAttribute('data-mermaid-source');
    const text = rawSource
        .replace(/&amp;/g, '&')
        .replace(/&quot;/g, '"')
        .replace(/&#39;/g, "'");
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
}};

// Step 3 implementation: single-diagram "Copy Image" button.
// Rasterizes SVG on demand and sends base64 PNG to the native copyImage
// message handler which writes it to NSPasteboard (see src/gui/view.rs).
window.copyMermaidImage = function(button) {{
    const container = button.closest('.mermaid-container');
    const svgEl = container.querySelector('.mermaid svg');
    if (!svgEl) return;

    const rect = svgEl.getBoundingClientRect();
    const scale = 2; // retina-quality output

    // Serialize SVG to a data URL (handles Unicode safely)
    const svgData = new XMLSerializer().serializeToString(svgEl);
    const svgBase64 = btoa(unescape(encodeURIComponent(svgData)));
    const dataUrl = 'data:image/svg+xml;base64,' + svgBase64;

    const img = new Image();
    img.onload = function() {{
        const canvas = document.createElement('canvas');
        canvas.width = rect.width * scale;
        canvas.height = rect.height * scale;
        const ctx = canvas.getContext('2d');

        // Fill with the container's background so dark/light themes look correct
        const bg = window.getComputedStyle(container.querySelector('.mermaid')).backgroundColor;
        ctx.fillStyle = bg || '#ffffff';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        ctx.scale(scale, scale);
        ctx.drawImage(img, 0, 0, rect.width, rect.height);

        const pngBase64 = canvas.toDataURL('image/png').split(',')[1];
        window.webkit.messageHandlers.copyImage.postMessage(pngBase64);
    }};
    img.onerror = function() {{
        console.error('Failed to rasterize SVG for image copy');
    }};
    img.src = dataUrl;
}};

// Step 1 implementation: rasterize SVG → retina PNG and store as data-png-src.
// Called automatically after each render so the synchronous copy handler
// (Step 2) can embed the image without async work.
window.cacheMermaidPng = function(container) {{
    const svgEl = container.querySelector('.mermaid svg');
    if (!svgEl) return;

    const rect = svgEl.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const scale = 2;

    const svgData = new XMLSerializer().serializeToString(svgEl);
    const svgBase64 = btoa(unescape(encodeURIComponent(svgData)));

    const img = new Image();
    img.onload = function() {{
        const canvas = document.createElement('canvas');
        canvas.width = rect.width * scale;
        canvas.height = rect.height * scale;
        const ctx = canvas.getContext('2d');

        const bg = window.getComputedStyle(container.querySelector('.mermaid')).backgroundColor;
        ctx.fillStyle = bg || '#ffffff';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        ctx.scale(scale, scale);
        ctx.drawImage(img, 0, 0, rect.width, rect.height);

        container.setAttribute('data-png-src', canvas.toDataURL('image/png'));
    }};
    img.src = 'data:image/svg+xml;base64,' + svgBase64;
}};

// Step 2 implementation: intercept the copy event, swap .mermaid-container
// nodes in the cloned selection for <img> tags with cached PNGs, and write
// the result to the clipboard. Falls through to native rich-text copy when
// no diagrams are in the selection.
document.addEventListener('copy', function(e) {{
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return;

    const frag = sel.getRangeAt(0).cloneContents();
    const wrap = document.createElement('div');
    wrap.appendChild(frag);

    const containers = wrap.querySelectorAll('.mermaid-container');
    if (containers.length === 0) return; // no diagrams — default rich text is fine

    containers.forEach(mc => {{
        const pngSrc = mc.getAttribute('data-png-src');
        if (!pngSrc) return; // cache not ready, leave SVG HTML as-is

        // Replace the entire container with a simple <img>
        const img = document.createElement('img');
        img.src = pngSrc;
        img.alt = 'Mermaid diagram';
        img.style.maxWidth = '100%';
        mc.replaceWith(img);
    }});

    e.clipboardData.setData('text/html', wrap.innerHTML);
    e.clipboardData.setData('text/plain', sel.toString());
    e.preventDefault();
}});

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
            // Step 1: pre-cache PNG for the copy event listener (see flow above)
            const mc = element.closest('.mermaid-container');
            if (mc) setTimeout(() => window.cacheMermaidPng(mc), 200);
        }} catch (error) {{
            console.error('Mermaid rendering error for appended content:', error);
            element.textContent = '';
            var errDiv = document.createElement('div');
            errDiv.style.cssText = 'color: red; padding: 10px; font-family: monospace;';
            errDiv.textContent = 'Mermaid rendering error: ' + error.message;
            element.appendChild(errDiv);
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
        vec!["https://cdn.jsdelivr.net/npm/mermaid@11.12.2/dist/mermaid.min.js".to_string()]
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
