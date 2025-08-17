use crate::content::{DocumentContent, ViewMode};
use crate::markdown;
use crate::plugins::{PluginContext, manager::PLUGIN_MANAGER};
use cacao::pasteboard::Pasteboard;
use cacao::webview::{InjectAt, WebView, WebViewConfig, WebViewDelegate};
use log::{debug, info};

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
}

const LINK_INTERCEPTOR_JS: &str = r#"
    window.addEventListener('DOMContentLoaded', (event) => {
        document.addEventListener('click', (e) => {
            let target = e.target.closest('a');
            if (target && target.href) {
                if (target.href.startsWith('http')) {
                    e.preventDefault();
                    window.webkit.messageHandlers.linkClicked.postMessage(target.href);
                }
            }
        });
        
        // Function to copy selected text
        window.copySelectedText = function() {
            const selectedText = window.getSelection().toString();
            if (selectedText.length > 0) {
                window.webkit.messageHandlers.copyText.postMessage(selectedText);
            }
        };
        
        // Function to select all text
        window.selectAllText = function() {
            const range = document.createRange();
            range.selectNodeContents(document.body);
            const selection = window.getSelection();
            selection.removeAllRanges();
            selection.addRange(range);
        };
        
        // Handle copy functionality
        document.addEventListener('keydown', (e) => {
            if (e.metaKey && e.key === 'c') {
                console.log('Copy key detected');
                const selectedText = window.getSelection().toString();
                console.log('Selected text length:', selectedText.length);
                if (selectedText.length > 0) {
                    e.preventDefault();
                    console.log('Sending copy message to handler');
                    window.webkit.messageHandlers.copyText.postMessage(selectedText);
                } else {
                    console.log('No text selected, allowing default behavior');
                }
            }
        });
        
        // Handle select all functionality
        document.addEventListener('keydown', (e) => {
            if (e.metaKey && e.key === 'a') {
                e.preventDefault();
                window.selectAllText();
            }
        });
        
        // Simple scroll functions
        window.scrollToBottom = function() {
            window.scrollTo(0, document.body.scrollHeight);
        };
        
        window.scrollToTop = function() {
            window.scrollTo(0, 0);
        };
        
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
                width: 44px;
                height: 44px;
                background: white;
                color: #333;
                border-radius: 50%;
                display: none;
                align-items: center;
                justify-content: center;
                cursor: pointer;
                font-size: 22px;
                font-weight: 900;
                z-index: 1000;
                transition: opacity 0.3s ease, transform 0.2s ease;
                user-select: none;
                box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15), 0 1px 3px rgba(0, 0, 0, 0.1);
                line-height: 44px;
                text-align: center;
                vertical-align: middle;
            `;
            
            // Add hover effect
            button.addEventListener('mouseenter', function() {
                this.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.2), 0 2px 6px rgba(0, 0, 0, 0.15)';
            });
            
            button.addEventListener('mouseleave', function() {
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
        
        // Track scroll state for showing/hiding during scroll
        window.scrollTimeout = null;
        window.hideTimeout = null;
        window.isScrolling = false;
        
        window.handleScroll = function() {
            const scrollButton = document.getElementById('scroll-to-bottom-btn');
            if (!scrollButton) return;
            
            const isNearBottom = (window.innerHeight + window.pageYOffset) >= (document.body.offsetHeight - 100);
            
            // Don't show button if near bottom
            if (isNearBottom) {
                scrollButton.style.opacity = '0';
                scrollButton.style.display = 'none';
                return;
            }
            
            // Show button when scrolling starts
            if (!window.isScrolling) {
                window.isScrolling = true;
                scrollButton.style.display = 'flex';
                setTimeout(function() {
                    scrollButton.style.opacity = '1';
                }, 10);
            }
            
            // Clear existing timeouts
            clearTimeout(window.scrollTimeout);
            clearTimeout(window.hideTimeout);
            
            // Set timeout to hide button after scrolling stops
            window.scrollTimeout = setTimeout(function() {
                window.isScrolling = false;
                
                // Hide button after a delay when scrolling stops
                window.hideTimeout = setTimeout(function() {
                    const isStillNearBottom = (window.innerHeight + window.pageYOffset) >= (document.body.offsetHeight - 100);
                    if (!isStillNearBottom && !window.isScrolling) {
                        scrollButton.style.opacity = '0';
                        setTimeout(function() {
                            if (scrollButton.style.opacity === '0') {
                                scrollButton.style.display = 'none';
                            }
                        }, 300);
                    }
                }, 2000); // Hide after 2 seconds of no scrolling
            }, 150); // 150ms after scroll stops
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
            
            // Initial button state check - don't show initially, let user scrolling trigger it
            // The button will appear when user starts scrolling
            
            // Set up a global handler for append operations
            window.handleAppendMessage = function(htmlContent) {
                window.appendContent(htmlContent);
            };
        });
    });
"#;

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

    // Get external CSS URLs
    let external_css = PLUGIN_MANAGER.get_all_external_css();
    let external_css_tags: Vec<String> = external_css
        .iter()
        .map(|url| format!(r#"<link rel="stylesheet" href="{url}">"#))
        .collect();

    html_parts.extend(external_css_tags);

    // Get external script URLs
    let external_scripts = PLUGIN_MANAGER.get_all_external_scripts();
    let external_script_tags: Vec<String> = external_scripts
        .iter()
        .map(|url| format!(r#"<script src="{url}"></script>"#))
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
                info!("Opening external link: {url}");
                open::that(url).ok();
            }
            "copyText" => {
                let text = body;
                info!("Copying text to clipboard: {} characters", text.len());
                debug!("Text content: '{text}'");

                // Copy to clipboard - try manual implementation
                let pasteboard = Pasteboard::default();
                pasteboard.clear_contents();

                // Try both the convenience method and manual approach
                pasteboard.copy_text(text);

                info!("Successfully copied to clipboard");
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
    accumulated_content: std::cell::RefCell<String>, // HTML content
    accumulated_markdown: std::cell::RefCell<String>, // Original markdown content
    last_sync_time: std::cell::RefCell<std::time::Instant>,
}

impl MarkdownView {
    /// Execute JavaScript in the WebView using native objc calls
    /// NOTE: Direct DOM appending (no reload) is crucial for streaming UX -
    /// reloading the entire page would interrupt user scrolling and selection
    #[allow(deprecated)]
    #[allow(unexpected_cfgs)]
    pub fn evaluate_javascript(&self, script: &str) {
        // Safely truncate very long scripts for logging (respecting Unicode boundaries)
        let script_preview = if script.len() > 200 {
            format!(
                "{}...(truncated {} chars)",
                safe_truncate(script, 200),
                script.len()
            )
        } else {
            script.to_string()
        };

        self.webview.objc.with_mut(|obj| unsafe {
            use cocoa::base::nil;
            use cocoa::foundation::NSString;
            use objc::{msg_send, sel, sel_impl};

            // Convert the script to NSString
            let ns_script = NSString::alloc(nil).init_str(script);

            // Call evaluateJavaScript:completionHandler: on the WKWebView
            let _: () = msg_send![obj, evaluateJavaScript:ns_script completionHandler:nil];

            debug!("Executed JavaScript: {script_preview}");
        });
    }

    pub fn new() -> Self {
        let mut config = WebViewConfig::default();
        config.add_handler("linkClicked");
        config.add_handler("copyText");
        config.add_handler("appendHTML");

        // CORRECTED: Use the correct enum variant `InjectAt::Start`.
        config.add_user_script(LINK_INTERCEPTOR_JS, InjectAt::Start, false);

        let delegate = LinkOpenerDelegate;
        let webview = WebView::with(config, delegate);

        MarkdownView {
            webview,
            current_mode: std::cell::RefCell::new(ViewMode::Preview),
            accumulated_content: std::cell::RefCell::new(String::new()),
            accumulated_markdown: std::cell::RefCell::new(String::new()),
            last_sync_time: std::cell::RefCell::new(std::time::Instant::now()),
        }
    }

    pub fn update_content(&self, document_content: &DocumentContent) {
        self.update_content_with_scroll(document_content, ScrollBehavior::Top);
    }

    pub fn append_content(
        &self,
        markdown_chunk: &str,
        html_chunk: &str,
        _style_preferences: &crate::gui::types::StylePreferences,
    ) {
        // Accumulate both markdown and HTML content
        self.accumulated_content.borrow_mut().push_str(html_chunk);
        self.accumulated_markdown
            .borrow_mut()
            .push_str(markdown_chunk);

        // Check if we need to do a periodic sync to ensure content integrity
        let now = std::time::Instant::now();
        let mut last_sync = self.last_sync_time.borrow_mut();
        let should_sync = now.duration_since(*last_sync) >= std::time::Duration::from_secs(5);

        // Only append to DOM if we're in preview mode
        if *self.current_mode.borrow() == ViewMode::Preview {
            if should_sync {
                // Periodic full refresh to ensure integrity
                debug!("Performing periodic content sync to ensure integrity");
                let full_content = self.accumulated_content.borrow().clone();
                let sync_script = format!(
                    r#"
                    try {{
                        // Clear and rebuild content to ensure integrity
                        document.body.innerHTML = {};
                        console.log('Periodic sync completed, content length:', document.body.innerHTML.length);
                        
                        // Re-initialize scroll button and plugins
                        if (typeof window.createScrollToBottomButton === 'function') {{
                            window.createScrollToBottomButton();
                            window.addEventListener('scroll', window.handleScroll);
                        }}
                        
                        if (typeof window.renderMermaidDiagrams === 'function') {{
                            window.renderMermaidDiagrams();
                        }}
                        if (typeof window.renderLatexExpressions === 'function') {{
                            window.renderLatexExpressions();
                        }}
                    }} catch(e) {{
                        console.error('Sync error:', e);
                    }}
                    "#,
                    serde_json::to_string(&full_content)
                        .unwrap_or_else(|_| "\"Sync error\"".to_string())
                );
                self.evaluate_javascript(&sync_script);
                *last_sync = now;
            } else {
                // Normal incremental append
                let json_escaped_html = serde_json::to_string(html_chunk)
                    .unwrap_or_else(|_| "\"Error: Could not escape HTML content\"".to_string());

                // Simplified append script that uses the queue system
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
        }
        // If we're in source mode, we'll regenerate the full content when toggling
    }

    pub fn update_content_with_scroll(
        &self,
        document_content: &DocumentContent,
        scroll_behavior: ScrollBehavior,
    ) {
        // Set accumulated content to match the new content (for streaming mode)
        *self.accumulated_content.borrow_mut() = document_content.html.clone();
        *self.accumulated_markdown.borrow_mut() = document_content.markdown.clone();
        *self.current_mode.borrow_mut() = document_content.mode.clone();

        let content = match document_content.mode {
            ViewMode::Preview => &document_content.html,
            ViewMode::Source => &markdown::highlight_markdown_with_theme(
                &document_content.markdown,
                &document_content.style_preferences.theme,
            ),
        };

        let onload_script = match scroll_behavior {
            ScrollBehavior::Bottom => "window.scrollToBottom();",
            ScrollBehavior::Top => "window.scrollToTop();",
        };

        let stylesheet = generate_stylesheet(document_content);
        let scripts = generate_scripts_html(document_content);
        let full_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>{stylesheet}</style>
    {scripts}
</head>
<body onload="{onload_script}">
{content}
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
        // For now, we rely on the JavaScript keyboard handler
        // This could be enhanced to directly trigger copy via JavaScript evaluation
        // if that API becomes available in future versions of cacao
        info!("Copy triggered via menu - use Cmd+C to copy selected text");
    }

    pub fn select_all_text(&self) {
        // For now, we rely on the JavaScript keyboard handler
        // This could be enhanced to directly trigger select all via JavaScript evaluation
        // if that API becomes available in future versions of cacao
        info!("Select All triggered via menu - use Cmd+A to select all text");
    }

    pub fn toggle_mode(&self, style_preferences: &crate::gui::types::StylePreferences) {
        // Toggle the current mode
        let new_mode = match *self.current_mode.borrow() {
            ViewMode::Preview => ViewMode::Source,
            ViewMode::Source => ViewMode::Preview,
        };
        *self.current_mode.borrow_mut() = new_mode.clone();

        // Regenerate content based on new mode using accumulated data
        let content = match new_mode {
            ViewMode::Preview => {
                // Use accumulated HTML content
                self.accumulated_content.borrow().clone()
            }
            ViewMode::Source => {
                // Generate highlighted markdown from accumulated markdown
                markdown::highlight_markdown_with_theme(
                    &self.accumulated_markdown.borrow(),
                    &style_preferences.theme,
                )
            }
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
            markdown: self.accumulated_markdown.borrow().clone(),
            html: content.clone(),
            mode: new_mode.clone(),
            title: "Toggle Mode".to_string(),
            file_path: None,
            style_preferences: style_preferences.clone(),
        });

        let onload_script = "window.scrollToTop();";
        let full_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>{stylesheet}</style>
    {scripts}
</head>
<body onload="{onload_script}">
{content}
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
