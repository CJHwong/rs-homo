use crate::content::DocumentContent;
use crate::gui::view::MarkdownView;
use cacao::appkit::window::{Window, WindowConfig, WindowStyle};
use cacao::appkit::App;

/// Calculates optimal window size based on content characteristics
fn calculate_window_size(content: &DocumentContent, is_pipe_mode: bool) -> (f64, f64) {
    let markdown_len = content.markdown.len();
    let line_count = content.markdown.lines().count();

    // Check if it's a markdown file based on content structure
    let is_markdown_file = content
        .file_path
        .as_ref()
        .map(|path| path.ends_with(".md") || path.ends_with(".markdown"))
        .unwrap_or(false)
        || content.markdown.contains("# ")
        || content.markdown.contains("## ")
        || content.markdown.contains("```");

    if is_pipe_mode {
        // Streaming content: minimal size
        (500.0, 400.0)
    } else if markdown_len < 500 || line_count < 10 {
        // Small content: minimal readable size
        (600.0, 450.0)
    } else if is_markdown_file {
        // Regular markdown file: comfortable reading size
        let width = if markdown_len > 5000 { 900.0 } else { 800.0 };
        let height = if line_count > 100 { 700.0 } else { 600.0 };
        (width, height)
    } else {
        // Default for other content
        (700.0, 500.0)
    }
}

/// Creates and configures the main application window for the markdown viewer.
pub fn create_main_window(content_view: &MarkdownView) -> Window {
    let mut config = WindowConfig::default();
    config.set_styles(&[
        WindowStyle::Titled,
        WindowStyle::Closable,
        WindowStyle::Resizable,
        WindowStyle::Miniaturizable,
    ]);

    let window = Window::new(config);

    window.set_title("Hoss' Opinionated Markdown Output");
    window.set_minimum_content_size(400., 300.);

    window.set_content_view(&content_view.webview);

    window.show();

    // Make sure the application becomes active and focused
    App::activate();

    window
}

/// Creates and configures the main application window with content-aware sizing.
pub fn create_main_window_with_content(
    content_view: &MarkdownView,
    content: &DocumentContent,
    is_pipe_mode: bool,
) -> Window {
    let mut config = WindowConfig::default();
    config.set_styles(&[
        WindowStyle::Titled,
        WindowStyle::Closable,
        WindowStyle::Resizable,
        WindowStyle::Miniaturizable,
    ]);

    let window = Window::new(config);

    window.set_title("Hoss' Opinionated Markdown Output");
    window.set_minimum_content_size(400., 300.);

    // Calculate and set content-aware window size
    let (width, height) = calculate_window_size(content, is_pipe_mode);
    window.set_content_size(width, height);

    window.set_content_view(&content_view.webview);

    window.show();

    // Make sure the application becomes active and focused
    App::activate();

    window
}
