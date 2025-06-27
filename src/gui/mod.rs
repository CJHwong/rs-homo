//! GUI module: sets up and runs the application window.

use cacao::appkit::App; // AppDelegate is not used directly here.
use std::sync::mpsc;

mod delegate;
mod view;
mod window;

pub use delegate::GuiDelegate;

/// Runs the GUI application, optionally with a receiver for streamed HTML.
pub fn run_app(receiver: Option<mpsc::Receiver<String>>) {
    App::new("com.rust-gui.homo", GuiDelegate::new(receiver)).run();
}
