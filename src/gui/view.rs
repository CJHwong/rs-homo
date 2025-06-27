use cacao::webview::{InjectAt, WebView, WebViewConfig, WebViewDelegate};

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
    });
"#;

const STYLESHEET: &str = r#"
:root {
    color-scheme: light dark;
}
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    line-height: 1.6;
    padding: 20px;
    margin: 0;
}
h1, h2, h3, h4, h5, h6 {
    border-bottom: 1px solid #d0d7de;
    padding-bottom: .3em;
    margin-top: 24px;
    margin-bottom: 16px;
}
code {
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    background-color: rgba(175, 184, 193, 0.2);
    padding: .2em .4em;
    margin: 0;
    font-size: 85%;
    border-radius: 6px;
}
pre {
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    background-color: #f6f8fa;
    padding: 16px;
    border-radius: 6px;
    overflow: auto;
}
pre > code {
    padding: 0;
    margin: 0;
    font-size: 100%;
    background-color: transparent;
    border: none;
}
blockquote {
    border-left: .25em solid #d0d7de;
    padding: 0 1em;
    color: #57606a;
}
"#;

#[derive(Default)]
pub struct LinkOpenerDelegate;

impl WebViewDelegate for LinkOpenerDelegate {
    fn on_message(&self, name: &str, body: &str) {
        if name == "linkClicked" {
            let url = body;
            println!("[INFO] Opening external link: {}", url);
            open::that(url).ok();
        }
    }
}

pub struct MarkdownView {
    pub webview: WebView<LinkOpenerDelegate>,
}

impl MarkdownView {
    pub fn new() -> Self {
        let mut config = WebViewConfig::default();
        config.add_handler("linkClicked");

        // CORRECTED: Use the correct enum variant `InjectAt::Start`.
        config.add_user_script(LINK_INTERCEPTOR_JS, InjectAt::Start, false);

        let delegate = LinkOpenerDelegate::default();
        let webview = WebView::with(config, delegate);

        MarkdownView { webview }
    }

    pub fn update_content(&self, html_content: &str) {
        let full_html = format!(
            "<!DOCTYPE html><html><head><meta charset=\"UTF-8\"><style>{}</style></head><body onload=\"window.scrollTo(0, document.body.scrollHeight);\">{}</body></html>",
            STYLESHEET, html_content
        );
        self.webview.load_html(&full_html);
    }
}
