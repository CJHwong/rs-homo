use crate::gui::view::MarkdownView;
use cacao::appkit::window::{Window, WindowConfig, WindowStyle};
use cacao::appkit::App;

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
