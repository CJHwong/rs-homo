use crate::content::{DocumentContent, ViewMode};
use crate::markdown;
use cacao::pasteboard::Pasteboard;
use cacao::webview::{InjectAt, WebView, WebViewConfig, WebViewDelegate};

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
    });
"#;

fn generate_stylesheet(content: &DocumentContent) -> String {
    content.style_preferences.generate_css()
}

#[derive(Default)]
pub struct LinkOpenerDelegate;

impl WebViewDelegate for LinkOpenerDelegate {
    fn on_message(&self, name: &str, body: &str) {
        println!(
            "[DEBUG] Received message: name='{}', body_len={}",
            name,
            body.len()
        );
        match name {
            "linkClicked" => {
                let url = body;
                println!("[INFO] Opening external link: {url}");
                open::that(url).ok();
            }
            "copyText" => {
                let text = body;
                println!(
                    "[INFO] Copying text to clipboard: {} characters",
                    text.len()
                );
                println!("[DEBUG] Text content: '{text}'");

                // Copy to clipboard - try manual implementation
                let pasteboard = Pasteboard::default();
                pasteboard.clear_contents();

                // Try both the convenience method and manual approach
                pasteboard.copy_text(text);

                println!("[INFO] Successfully copied to clipboard");
            }
            _ => {
                println!("[DEBUG] Unknown message type: {name}");
            }
        }
    }
}

pub struct MarkdownView {
    pub webview: WebView<LinkOpenerDelegate>,
    current_mode: std::cell::RefCell<ViewMode>,
}

impl MarkdownView {
    pub fn new() -> Self {
        let mut config = WebViewConfig::default();
        config.add_handler("linkClicked");
        config.add_handler("copyText");

        // CORRECTED: Use the correct enum variant `InjectAt::Start`.
        config.add_user_script(LINK_INTERCEPTOR_JS, InjectAt::Start, false);

        let delegate = LinkOpenerDelegate;
        let webview = WebView::with(config, delegate);

        MarkdownView {
            webview,
            current_mode: std::cell::RefCell::new(ViewMode::Preview),
        }
    }

    pub fn update_content(&self, document_content: &DocumentContent) {
        self.update_content_with_scroll(document_content, ScrollBehavior::Top);
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

        let onload_script = match scroll_behavior {
            ScrollBehavior::Bottom => "window.scrollToBottom();",
            ScrollBehavior::Top => "window.scrollToTop();",
        };

        let stylesheet = generate_stylesheet(document_content);
        let full_html = format!(
            "<!DOCTYPE html><html><head><meta charset=\"UTF-8\"><style>{stylesheet}</style></head><body onload=\"{onload_script}\">{content}</body></html>"
        );
        self.webview.load_html(&full_html);
    }

    pub fn copy_selected_text(&self) {
        // For now, we rely on the JavaScript keyboard handler
        // This could be enhanced to directly trigger copy via JavaScript evaluation
        // if that API becomes available in future versions of cacao
        println!("[INFO] Copy triggered via menu - use Cmd+C to copy selected text");
    }

    pub fn select_all_text(&self) {
        // For now, we rely on the JavaScript keyboard handler
        // This could be enhanced to directly trigger select all via JavaScript evaluation
        // if that API becomes available in future versions of cacao
        println!("[INFO] Select All triggered via menu - use Cmd+A to select all text");
    }
}
