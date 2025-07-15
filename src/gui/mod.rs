//! GUI module: sets up and runs the application window.

use crate::content::DocumentContent;
use cacao::appkit::App; // AppDelegate is not used directly here.
use std::sync::mpsc;

mod delegate;
pub mod types;
mod view;
mod window;

pub use delegate::GuiDelegate;

/// Runs the GUI application, optionally with a receiver for streamed DocumentContent.
pub fn run_app(receiver: Option<mpsc::Receiver<DocumentContent>>, is_pipe_mode: bool) {
    App::new(
        "com.rust-gui.homo",
        GuiDelegate::new(receiver, is_pipe_mode),
    )
    .run();
}
