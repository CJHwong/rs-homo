//! Implements the application delegate for the GUI lifecycle and message handling.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self};

use cacao::appkit::window::Window;
use cacao::appkit::AppDelegate;

use crate::gui::view::MarkdownView;
use crate::gui::window::create_main_window;

/// Handles the main window and markdown content updates.
pub struct GuiDelegate {
    window: RefCell<Option<Window>>,
    view: Rc<MarkdownView>,
    receiver: Option<mpsc::Receiver<String>>,
}

impl GuiDelegate {
    /// Creates a new GUI delegate with an optional receiver for streamed HTML.
    pub fn new(receiver: Option<mpsc::Receiver<String>>) -> Self {
        GuiDelegate {
            window: RefCell::new(None),
            view: Rc::new(MarkdownView::new()),
            receiver,
        }
    }
}

impl AppDelegate for GuiDelegate {
    /// Called periodically; handles window creation and content updates.
    fn did_update(&self) {
        // Check if we have a message receiver (i.e., we're in pipe mode).
        if let Some(receiver) = &self.receiver {
            // Check for a message without blocking.
            if let Ok(html_content) = receiver.try_recv() {
                // If the window doesn't exist yet, this is the first message.
                if self.window.borrow().is_none() {
                    println!("[INFO] First message received. Creating window...");
                    let window = create_main_window(&self.view);
                    self.view.update_content(&html_content);
                    // Store the window to prevent creating it again.
                    *self.window.borrow_mut() = Some(window);
                } else {
                    // Window already exists, just update its content.
                    self.view.update_content(&html_content);
                }
            }
        } else {
            // This is the non-pipe mode. Create a window on the first tick.
            if self.window.borrow().is_none() {
                println!("[INFO] No pipe detected. Creating empty window...");
                let window = create_main_window(&self.view);
                *self.window.borrow_mut() = Some(window);
            }
        }
    }

    /// Prevents the framework from opening an automatic "Untitled" window.
    fn should_open_untitled_file(&self) -> bool {
        false
    }

    /// Ensures the app terminates after the last window is closed.
    fn should_terminate_after_last_window_closed(&self) -> bool {
        true
    }
}
