//! Implements the application delegate for the GUI lifecycle and message handling.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self};

use cacao::appkit::window::Window;
use cacao::appkit::{App, AppDelegate};

use crate::content::DocumentContent;
use crate::gui::view::{MarkdownView, ScrollBehavior};
use crate::gui::window::create_main_window;
use crate::menu::{self, MenuMessage};

/// Handles the main window and markdown content updates.
pub struct GuiDelegate {
    window: RefCell<Option<Window>>,
    view: Rc<MarkdownView>,
    receiver: Option<mpsc::Receiver<DocumentContent>>,
    menu_setup: RefCell<bool>,
    current_document: RefCell<Option<DocumentContent>>,
    menu_receiver: RefCell<Option<mpsc::Receiver<MenuMessage>>>,
    is_pipe_mode: bool,
}

impl GuiDelegate {
    /// Creates a new GUI delegate with an optional receiver for streamed DocumentContent.
    pub fn new(receiver: Option<mpsc::Receiver<DocumentContent>>, is_pipe_mode: bool) -> Self {
        // Set up menu message channel
        let (menu_sender, menu_receiver) = mpsc::channel();
        menu::set_menu_sender(menu_sender);

        GuiDelegate {
            window: RefCell::new(None),
            view: Rc::new(MarkdownView::new()),
            receiver,
            menu_setup: RefCell::new(false),
            current_document: RefCell::new(None),
            menu_receiver: RefCell::new(Some(menu_receiver)),
            is_pipe_mode,
        }
    }

    /// Set up the main menu for the application
    fn setup_menu(&self) {
        if *self.menu_setup.borrow() {
            return; // Menu already setup
        }

        // Set the complete menu bar for the application
        App::set_menu(menu::create_menus());

        *self.menu_setup.borrow_mut() = true;
    }

    /// Handles the toggle mode action
    pub fn toggle_mode(&self) {
        let mut current_document_option = self.current_document.borrow_mut();
        if let Some(current_document) = current_document_option.as_mut() {
            current_document.toggle_mode();
            self.view.update_content(current_document);
        }
    }
}

impl AppDelegate for GuiDelegate {
    /// Called when the application finishes launching.
    fn did_finish_launching(&self) {
        // Menu setup is now handled when the first window is created
    }

    /// Called periodically; handles window creation and content updates.
    fn did_update(&self) {
        // Handle menu messages
        if let Some(menu_receiver) = self.menu_receiver.borrow().as_ref() {
            if let Ok(menu_message) = menu_receiver.try_recv() {
                match menu_message {
                    MenuMessage::ToggleMode => {
                        self.toggle_mode();
                    }
                    MenuMessage::Copy => {
                        self.view.copy_selected_text();
                    }
                    MenuMessage::SelectAll => {
                        self.view.select_all_text();
                    }
                }
            }
        }

        // Check if we have a message receiver (i.e., we're in pipe mode).
        if let Some(receiver) = &self.receiver {
            // Check for a message without blocking.
            if let Ok(document_content) = receiver.try_recv() {
                // If the window doesn't exist yet, this is the first message.
                if self.window.borrow().is_none() {
                    println!("[INFO] First message received. Creating window...");
                    self.setup_menu(); // Set up menu when creating first window
                    let window = create_main_window(&self.view);

                    // Choose scroll behavior based on mode
                    let scroll_behavior = if self.is_pipe_mode {
                        ScrollBehavior::Bottom // Pipe mode: scroll to bottom
                    } else {
                        ScrollBehavior::Top // File mode: stay at top
                    };

                    self.view
                        .update_content_with_scroll(&document_content, scroll_behavior);
                    // Store the window to prevent creating it again.
                    *self.window.borrow_mut() = Some(window);
                } else {
                    // Window already exists, just update its content.

                    // Choose scroll behavior based on mode
                    let scroll_behavior = if self.is_pipe_mode {
                        ScrollBehavior::Bottom // Pipe mode: scroll to bottom
                    } else {
                        ScrollBehavior::Top // File mode: stay at top
                    };

                    self.view
                        .update_content_with_scroll(&document_content, scroll_behavior);
                }
                // Store the current document content
                *self.current_document.borrow_mut() = Some(document_content);
            }
        } else {
            // This is the non-pipe mode. Create a window on the first tick.
            if self.window.borrow().is_none() {
                println!("[INFO] No pipe detected. Creating empty window...");
                self.setup_menu(); // Set up menu when creating first window
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
