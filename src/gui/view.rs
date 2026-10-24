use std::sync::atomic::{AtomicU64, Ordering};

use crate::content::{DocumentContent, ViewMode};
use crate::markdown;
use crate::plugins::{PluginContext, manager::PLUGIN_MANAGER};
use cacao::webview::{InjectAt, WebView, WebViewConfig, WebViewDelegate};
use log::{debug, info};

/// Stores the last known scroll ratio (0.0–1.0) as AtomicU64 bits so it can
/// be written from the message-handler thread and read from the main thread.
static SCROLL_RATIO: AtomicU64 = AtomicU64::new(0);

fn set_scroll_ratio(ratio: f64) {
    SCROLL_RATIO.store(ratio.to_bits(), Ordering::Relaxed);
}

fn get_scroll_ratio() -> f64 {
    f64::from_bits(SCROLL_RATIO.load(Ordering::Relaxed))
}

/// Safely truncate a string at the given byte limit, respecting Unicode character boundaries
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    let safe_end = s
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| i >= max_bytes)
        .unwrap_or(s.len());

    &s[..safe_end]
}

#[derive(Clone, Copy)]
pub enum ScrollBehavior {
    Top,
    Bottom,
    Preserve,
}

/// JavaScript injected into every page load to handle WebView interactions.
///
/// ## Rich Text Copy Architecture
///
/// Copy uses a two-layer approach so that Cmd+A → Cmd+C produces a rich
/// document with Mermaid diagrams embedded as images:
///
/// 1. **Text & basic HTML** — WKWebView's native copy handles this. We do NOT
///    intercept Cmd+C for normal content; the browser writes both `text/html`
///    and `text/plain` to the pasteboard automatically.
///
/// 2. **Mermaid diagrams** — Handled by the Mermaid plugin's JS (see
///    `src/plugins/mermaid.rs`). After each diagram renders, its SVG is
///    rasterized to a retina PNG and cached on the container element as a
///    `data-png-src` attribute. A `document.addEventListener('copy', ...)`
///    handler then clones the selection, swaps any `.mermaid-container` nodes
///    for `<img>` tags using the cached PNGs, and writes the modified HTML to
///    the clipboard. If no Mermaid containers are in the selection, the handler
///    returns early and the native copy proceeds unchanged.
///
/// 3. **Single-diagram image copy** — The "Copy Image" button on each diagram
///    sends base64 PNG data to the native `copyImage` message handler, which
///    writes it to `NSPasteboard` as `public.png` (see `LinkOpenerDelegate`).
const LINK_INTERCEPTOR_JS: &str = r#"
    window.addEventListener('DOMContentLoaded', (event) => {
        document.addEventListener('click', (e) => {
            let target = e.target.closest('a');
            if (target) {
                const href = target.getAttribute('href');
                if (href && href.startsWith('#')) {
                    e.preventDefault();
                    const targetId = decodeURIComponent(href.substring(1));
                    const targetElement = document.getElementById(targetId);
                    if (targetElement) {
                        targetElement.scrollIntoView({ behavior: 'smooth' });
                    }
                    return;
                }
                if (target.href && target.href.startsWith('http')) {
                    e.preventDefault();
                    window.webkit.messageHandlers.linkClicked.postMessage(target.href);
                }
            }
        });
        
        // Select all: scope to document body so margins/chrome are excluded
        window.selectAllText = function() {
            const range = document.createRange();
            range.selectNodeContents(document.body);
            const selection = window.getSelection();
            selection.removeAllRanges();
            selection.addRange(range);
        };

        document.addEventListener('keydown', (e) => {
            if (e.metaKey && e.key === 'a') {
                e.preventDefault();
                window.selectAllText();
            }
        });

        // Disable default WebView context menu (its "Reload" option blanks the page
        // since content is loaded via loadHTMLString, not from a URL)
        document.addEventListener('contextmenu', (e) => e.preventDefault());
        
        // Simple scroll functions
        window.scrollToBottom = function() {
            window.scrollTo(0, document.body.scrollHeight);
        };

        window.scrollToTop = function() {
            window.scrollTo(0, 0);
        };

        // Restore scroll position by ratio (0..1).
        // Body starts hidden (opacity:0 set on the <body> tag) so the user
        // never sees an intermediate wrong position.  We re-apply the ratio
        // as plugin rendering changes page height, then reveal once stable.
        window.scrollToRatio = function(r) {
            __scrollRatio = r;
            var apply = function() {
                var max = document.body.scrollHeight - window.innerHeight;
                if (max > 0) window.scrollTo(0, max * r);
            };
            apply();
            var lastHeight = document.body.scrollHeight;
            var checks = 0;
            var stableCount = 0;
            var settle = setInterval(function() {
                var h = document.body.scrollHeight;
                if (h !== lastHeight) {
                    lastHeight = h;
                    stableCount = 0;
                    apply();
                } else {
                    stableCount++;
                }
                checks++;
                if (stableCount >= 2 || checks >= 10) {
                    clearInterval(settle);
                    apply();
                    if (typeof window.revealContent === 'function') {
                        window.revealContent();
                    } else {
                        document.body.style.opacity = '1';
                    }
                }
            }, 50);
        };

        // --- Scroll ratio tracking (sent to native for reload preservation) ---
        var __scrollRatio = 0;
        var __scrollThrottle = null;
        var __resizing = false;

        window.addEventListener('scroll', function() {
            if (__resizing) return;
            var max = document.body.scrollHeight - window.innerHeight;
            __scrollRatio = max > 0 ? window.scrollY / max : 0;
            if (!__scrollThrottle) {
                __scrollThrottle = setTimeout(function() {
                    __scrollThrottle = null;
                    window.webkit.messageHandlers.scrollRatio.postMessage(
                        __scrollRatio.toString()
                    );
                }, 150);
            }
        });

        window.addEventListener('resize', function() {
            __resizing = true;
            requestAnimationFrame(function() {
                var max = document.body.scrollHeight - window.innerHeight;
                if (max > 0) window.scrollTo(0, max * __scrollRatio);
                requestAnimationFrame(function() { __resizing = false; });
            });
        });
        
        // Create scroll to bottom button
        window.createScrollToBottomButton = function() {
            const button = document.createElement('div');
            button.id = 'scroll-to-bottom-btn';
            button.innerHTML = '↓';
            button.style.cssText = `
                position: fixed;
                bottom: 16px;
                left: 50%;
                transform: translateX(-50%);
                width: 32px;
                height: 32px;
                background: #f6f6f6;
                color: var(--muted-text-color, #333);
                border-radius: 50%;
                display: none;
                align-items: center;
                justify-content: center;
                cursor: pointer;
                font-size: 14px;
                font-weight: 900;
                z-index: 1000;
                transition: opacity 0.3s ease, transform 0.2s ease;
                user-select: none;
                box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15), 0 1px 3px rgba(0, 0, 0, 0.1);
                line-height: 1;
                text-align: center;
            `;
            
            // Add hover effect
            button.addEventListener('mouseenter', function() {
                this.style.background = '#fefefe';
                this.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.2), 0 2px 6px rgba(0, 0, 0, 0.15)';
            });

            button.addEventListener('mouseleave', function() {
                this.style.background = '#f6f6f6';
                this.style.boxShadow = '0 2px 8px rgba(0, 0, 0, 0.15), 0 1px 3px rgba(0, 0, 0, 0.1)';
            });
            
            button.addEventListener('click', function() {
                window.scrollTo({
                    top: document.body.scrollHeight,
                    behavior: 'smooth'
                });
            });
            
            document.body.appendChild(button);
            return button;
        };
        
        // Function to check if user is near bottom and show/hide button
        window.updateScrollButton = function() {
            const scrollButton = document.getElementById('scroll-to-bottom-btn');
            if (!scrollButton) return;
            
            const isNearBottom = (window.innerHeight + window.pageYOffset) >= (document.body.offsetHeight - 100);
            
            if (isNearBottom) {
                scrollButton.style.opacity = '0';
                setTimeout(function() {
                    if (scrollButton.style.opacity === '0') {
                        scrollButton.style.display = 'none';
                    }
                }, 300); // Match the transition duration
            } else {
                scrollButton.style.display = 'flex';
                setTimeout(function() {
                    scrollButton.style.opacity = '1';
                }, 10); // Small delay to ensure display change happens first
            }
        };
        
        window.handleScroll = function() {
            window.updateScrollButton();
        };

        // Initialize append queue system for sequential processing with retry mechanism
        window.appendQueue = [];
        window.isProcessingQueue = false;
        window.appendStats = { processed: 0, failed: 0 };
        
        // Process the next item in the append queue with robust error handling
        window.processNextAppend = function() {
            if (window.appendQueue.length === 0) {
                window.isProcessingQueue = false;
                console.log('Queue empty. Stats:', window.appendStats);
                return;
            }
            
            window.isProcessingQueue = true;
            const queueItem = window.appendQueue.shift();
            const { htmlContent, retryCount = 0 } = queueItem;
            
            try {
                // Verify DOM is in a good state before appending
                if (!document.body) {
                    console.error('Document body not available, requeueing...');
                    window.appendQueue.unshift({ htmlContent, retryCount: retryCount + 1 });
                    setTimeout(window.processNextAppend, 100);
                    return;
                }
                
                const startTime = performance.now();
                window.doAppendContent(htmlContent);
                const endTime = performance.now();
                
                window.appendStats.processed++;
                console.log(`Append ${window.appendStats.processed} completed in ${(endTime - startTime).toFixed(2)}ms`);
                
                // Process next item with adaptive delay based on processing time
                const delay = endTime - startTime > 50 ? 20 : 5;
                setTimeout(window.processNextAppend, delay);
                
            } catch (error) {
                console.error('Error in processNextAppend:', error);
                window.appendStats.failed++;
                
                // Retry mechanism for failed appends
                if (retryCount < 3) {
                    console.log(`Retrying append (attempt ${retryCount + 1}/3)...`);
                    window.appendQueue.unshift({ htmlContent, retryCount: retryCount + 1 });
                    setTimeout(window.processNextAppend, 50 * (retryCount + 1));
                } else {
                    console.error('Max retries exceeded, skipping content:', htmlContent.substring(0, 100));
                    // The DOM is now missing a chunk; ask the app for a full reload.
                    window.webkit.messageHandlers.appendFailed.postMessage('');
                    setTimeout(window.processNextAppend, 10);
                }
            }
        };
        
        // Queue-based content appending function with immediate processing trigger
        window.appendContent = function(htmlContent) {
            // Store as object to support retry metadata
            window.appendQueue.push({ htmlContent, retryCount: 0 });
            console.log(`Queued content, queue size: ${window.appendQueue.length}`);
            
            if (!window.isProcessingQueue) {
                // Use requestAnimationFrame for better timing with rendering
                requestAnimationFrame(window.processNextAppend);
            }
        };

        // Core content appending function (synchronous)
        window.doAppendContent = function(htmlContent) {
            // Check if user was near the bottom before adding content
            const wasNearBottom = (window.innerHeight + window.pageYOffset) >= (document.body.offsetHeight - 300);
            
            const div = document.createElement('div');
            div.innerHTML = htmlContent;
            document.body.appendChild(div);
            
            // Only scroll to bottom if user was already near the bottom
            if (wasNearBottom) {
                window.scrollTo({
                    top: document.body.scrollHeight,
                    behavior: 'smooth'
                });
            }
            
            // Update scroll button visibility after content change
            window.updateScrollButton();

            // Re-initialize plugins for any new content
            if (typeof window.renderNewMermaidDiagrams === 'function') {
                window.renderNewMermaidDiagrams(div);
            }
            if (typeof window.renderNewLatexExpressions === 'function') {
                window.renderNewLatexExpressions(div);
            }
        };
        
        // Initialize everything when DOM is ready
        document.addEventListener('DOMContentLoaded', function() {
            console.log('DOM loaded, initializing scroll button...');
            
            // Create the scroll to bottom button
            const scrollButton = window.createScrollToBottomButton();
            console.log('Scroll button created:', scrollButton);
            
            // Set up scroll event listener to show/hide button with fade during scroll
            window.addEventListener('scroll', function() {
                window.handleScroll();
            });
            
            // Show button immediately if content is scrollable
            setTimeout(function() {
                window.updateScrollButton();
            }, 100);
            
            // Set up a global handler for append operations
            window.handleAppendMessage = function(htmlContent) {
                window.appendContent(htmlContent);
            };
        });
    });
"#;

use super::crab;

fn generate_stylesheet(content: &DocumentContent) -> String {
    let base_css = content.style_preferences.generate_css();

    // Get plugin CSS
    let context = PluginContext {
        theme_mode: content.style_preferences.theme.clone(),
        is_streaming: false,
        content_id: "main".to_string(),
    };

    let plugin_css = PLUGIN_MANAGER.get_all_css(&context);

    if plugin_css.is_empty() {
        base_css
    } else {
        format!("{base_css}\n\n/* Plugin Styles */\n{plugin_css}")
    }
}

fn generate_scripts_html(content: &DocumentContent) -> String {
    let context = PluginContext {
        theme_mode: content.style_preferences.theme.clone(),
        is_streaming: false,
        content_id: "main".to_string(),
    };

    let mut html_parts = Vec::new();

    // Get external CSS URLs with SRI integrity hashes
    let external_css = PLUGIN_MANAGER.get_all_external_css_with_integrity();
    let external_css_tags: Vec<String> = external_css
        .iter()
        .map(|(url, integrity)| {
            if integrity.is_empty() {
                format!(r#"<link rel="stylesheet" href="{url}">"#)
            } else {
                format!(
                    r#"<link rel="stylesheet" href="{url}" integrity="{integrity}" crossorigin="anonymous">"#
                )
            }
        })
        .collect();

    html_parts.extend(external_css_tags);

    // Get external script URLs with SRI integrity hashes
    let external_scripts = PLUGIN_MANAGER.get_all_external_scripts_with_integrity();
    let external_script_tags: Vec<String> = external_scripts
        .iter()
        .map(|(url, integrity)| {
            if integrity.is_empty() {
                format!(r#"<script src="{url}"></script>"#)
            } else {
                format!(
                    r#"<script src="{url}" integrity="{integrity}" crossorigin="anonymous"></script>"#
                )
            }
        })
        .collect();

    html_parts.extend(external_script_tags);

    // Get plugin JavaScript
    let plugin_js = PLUGIN_MANAGER.get_all_javascript(&context);

    if !plugin_js.is_empty() {
        html_parts.push(format!("<script>\n{plugin_js}\n</script>"));
    }

    html_parts.join("\n")
}

#[derive(Default)]
pub struct LinkOpenerDelegate;

impl WebViewDelegate for LinkOpenerDelegate {
    fn on_message(&self, name: &str, body: &str) {
        debug!("Received message: name='{}', body_len={}", name, body.len());
        match name {
            "linkClicked" => {
                let url = body;
                if url.starts_with("http://") || url.starts_with("https://") {
                    info!("Opening external link: {url}");
                    open::that(url).ok();
                } else {
                    debug!("Blocked non-HTTP URL scheme: {url}");
                }
            }
            // Receives base64-encoded PNG data from the Mermaid plugin's
            // "Copy Image" button and writes it to NSPasteboard as public.png.
            // Uses NSData's built-in base64 decoding to avoid extra dependencies.
            "copyImage" => {
                info!("Copying image to clipboard ({} bytes base64)", body.len());
                #[allow(deprecated)]
                unsafe {
                    use cocoa::base::{id, nil};
                    use cocoa::foundation::NSString;
                    use objc::{class, msg_send, sel, sel_impl};

                    let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
                    let _: i64 = msg_send![pasteboard, clearContents];

                    let ns_base64 = NSString::alloc(nil).init_str(body);
                    let ns_data: id = msg_send![class!(NSData), alloc];
                    let ns_data: id =
                        msg_send![ns_data, initWithBase64EncodedString:ns_base64 options:0u64];

                    if ns_data != nil {
                        let png_type = NSString::alloc(nil).init_str("public.png");
                        let ok: bool = msg_send![pasteboard, setData:ns_data forType:png_type];
                        if ok {
                            info!("Successfully copied image to clipboard");
                        } else {
                            debug!("Failed to write image data to pasteboard");
                        }
                    } else {
                        debug!("Failed to decode base64 image data");
                    }
                }
            }
            "appendFailed" => {
                info!("WebView dropped a chunk; requesting a full reload");
                crate::menu::dispatch_menu_message(crate::menu::MenuMessage::ReloadDocument);
            }
            "scrollRatio" => {
                if let Ok(ratio) = body.parse::<f64>() {
                    set_scroll_ratio(ratio);
                    debug!("Scroll ratio updated: {ratio:.4}");
                }
            }
            _ => {
                debug!("Unknown message type: {name}");
            }
        }
    }
}

pub struct MarkdownView {
    pub webview: WebView<LinkOpenerDelegate>,
    current_mode: std::cell::RefCell<ViewMode>,
}

impl MarkdownView {
    /// Execute JavaScript in the WebView using native objc calls
    /// NOTE: Direct DOM appending (no reload) is crucial for streaming UX -
    /// reloading the entire page would interrupt user scrolling and selection
    #[allow(deprecated)]
    #[allow(unexpected_cfgs)]
    pub fn evaluate_javascript(&self, script: &str) {
        self.webview.objc.with_mut(|obj| unsafe {
            use cocoa::base::nil;
            use cocoa::foundation::NSString;
            use objc::{msg_send, sel, sel_impl};

            // alloc/init returns a +1 reference; WKWebView keeps its own copy.
            let ns_script = NSString::alloc(nil).init_str(script);
            let _: () = msg_send![obj, evaluateJavaScript:ns_script completionHandler:nil];
            let _: () = msg_send![ns_script, release];
        });

        if log::log_enabled!(log::Level::Debug) {
            // Truncate very long scripts for logging (respecting Unicode boundaries)
            let suffix = if script.len() > 200 {
                "...(truncated)"
            } else {
                ""
            };
            debug!(
                "Executed JavaScript ({} bytes): {}{suffix}",
                script.len(),
                safe_truncate(script, 200)
            );
        }
    }

    pub fn new() -> Self {
        let mut config = WebViewConfig::default();
        config.add_handler("linkClicked");
        config.add_handler("copyImage");
        config.add_handler("appendHTML");
        config.add_handler("scrollRatio");
        config.add_handler("appendFailed");

        // CORRECTED: Use the correct enum variant `InjectAt::Start`.
        config.add_user_script(LINK_INTERCEPTOR_JS, InjectAt::Start, false);

        let delegate = LinkOpenerDelegate;
        let webview = WebView::with(config, delegate);

        MarkdownView {
            webview,
            current_mode: std::cell::RefCell::new(ViewMode::Preview),
        }
    }

    /// Append one parsed chunk to the page. The caller keeps the document;
    /// source mode shows nothing new until the next toggle rebuilds it.
    pub fn append_content(&self, html_chunk: &str) {
        if *self.current_mode.borrow() != ViewMode::Preview {
            return;
        }
        let json_escaped_html = serde_json::to_string(html_chunk)
            .unwrap_or_else(|_| "\"Error: Could not escape HTML content\"".to_string());

        let append_script = format!(
            r#"
            try {{
                if (typeof window.appendContent === 'function') {{
                    window.appendContent({json_escaped_html});
                }} else {{
                    console.error('appendContent function not available');
                }}
            }} catch(e) {{
                console.error('JavaScript append error:', e);
            }}
            "#
        );

        debug!(
            "Queuing content append with {} characters of HTML",
            html_chunk.len()
        );
        self.evaluate_javascript(&append_script);
    }

    pub fn update_content_with_scroll(
        &self,
        document_content: &DocumentContent,
        scroll_behavior: ScrollBehavior,
    ) {
        *self.current_mode.borrow_mut() = document_content.mode.clone();

        let content = match document_content.mode {
            ViewMode::Preview => &document_content.html,
            ViewMode::Source => &markdown::highlight_markdown_with_theme(
                &document_content.markdown,
                &document_content.style_preferences.theme,
            ),
        };

        let is_preserve = matches!(scroll_behavior, ScrollBehavior::Preserve);
        let onload_script = match scroll_behavior {
            ScrollBehavior::Bottom => "window.scrollToBottom();".to_string(),
            ScrollBehavior::Top => "window.scrollToTop();".to_string(),
            ScrollBehavior::Preserve => {
                let ratio = get_scroll_ratio();
                format!("window.scrollToRatio({ratio});")
            }
        };
        let crab_css = if is_preserve {
            crab::overlay_css()
        } else {
            String::new()
        };
        let crab_js = if is_preserve {
            crab::reveal_js(400)
        } else {
            String::new()
        };
        let crab_overlay = if is_preserve {
            crab::overlay_html()
        } else {
            String::new()
        };
        let wrap_open = if is_preserve {
            crab::CONTENT_WRAP_OPEN
        } else {
            ""
        };
        let wrap_close = if is_preserve {
            crab::CONTENT_WRAP_CLOSE
        } else {
            ""
        };

        let stylesheet = generate_stylesheet(document_content);
        let scripts = generate_scripts_html(document_content);
        let full_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>{stylesheet}{crab_css}</style>
    {scripts}
    {crab_js}
</head>
<body onload="{onload_script}">
{crab_overlay}{wrap_open}
{content}
{wrap_close}
<script>
// Initialize scroll to bottom button for regular content updates
setTimeout(function() {{
    console.log('Trying to create scroll button...');
    if (typeof window.createScrollToBottomButton === 'function') {{
        console.log('Creating scroll button from inline script...');
        window.createScrollToBottomButton();
        window.addEventListener('scroll', function() {{
            window.handleScroll();
        }});
        setTimeout(function() {{
            window.updateScrollButton();
        }}, 100);
    }} else {{
        console.log('createScrollToBottomButton function not available');
    }}
}}, 200);
</script>
</body>
</html>"#
        );
        self.webview.load_html(&full_html);
    }

    pub fn copy_selected_text(&self) {
        self.evaluate_javascript("document.execCommand('copy')");
    }

    pub fn select_all_text(&self) {
        self.evaluate_javascript("window.selectAllText()");
    }

    pub fn toggle_mode(
        &self,
        document: &DocumentContent,
        style_preferences: &crate::gui::types::StylePreferences,
    ) {
        // Toggle the current mode
        let new_mode = match *self.current_mode.borrow() {
            ViewMode::Preview => ViewMode::Source,
            ViewMode::Source => ViewMode::Preview,
        };
        *self.current_mode.borrow_mut() = new_mode.clone();

        let content = match new_mode {
            ViewMode::Preview => document.html.clone(),
            ViewMode::Source => markdown::highlight_markdown_with_theme(
                &document.markdown,
                &style_preferences.theme,
            ),
        };

        // Do a full reload for mode toggle (this is acceptable since it's user-initiated)
        let base_css = style_preferences.generate_css();
        let context = PluginContext {
            theme_mode: style_preferences.theme.clone(),
            is_streaming: false,
            content_id: "toggle".to_string(),
        };

        let plugin_css = PLUGIN_MANAGER.get_all_css(&context);
        let stylesheet = if plugin_css.is_empty() {
            base_css
        } else {
            format!("{base_css}\n\n/* Plugin Styles */\n{plugin_css}")
        };

        let scripts = generate_scripts_html(&DocumentContent {
            markdown: document.markdown.clone(),
            html: content.clone(),
            mode: new_mode.clone(),
            title: "Toggle Mode".to_string(),
            file_path: None,
            style_preferences: style_preferences.clone(),
        });

        let ratio = get_scroll_ratio();
        let onload_script = format!("window.scrollToRatio({ratio});");
        let crab_css = crab::overlay_css();
        let crab_js = crab::reveal_js(400);
        let crab_overlay = crab::overlay_html();
        let wrap_open = crab::CONTENT_WRAP_OPEN;
        let wrap_close = crab::CONTENT_WRAP_CLOSE;
        let full_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>{stylesheet}{crab_css}</style>
    {scripts}
    {crab_js}
</head>
<body onload="{onload_script}">
{crab_overlay}
{wrap_open}
{content}
{wrap_close}
<script>
// Initialize scroll to bottom button for mode toggle
setTimeout(function() {{
    console.log('Trying to create scroll button in mode toggle...');
    if (typeof window.createScrollToBottomButton === 'function') {{
        console.log('Creating scroll button from mode toggle...');
        window.createScrollToBottomButton();
        window.addEventListener('scroll', function() {{
            window.handleScroll();
        }});
        setTimeout(function() {{
            window.updateScrollButton();
        }}, 100);
    }} else {{
        console.log('createScrollToBottomButton function not available in mode toggle');
    }}
}}, 200);
</script>
</body>
</html>"#
        );
        self.webview.load_html(&full_html);
    }
}
