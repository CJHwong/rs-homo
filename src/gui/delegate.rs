//! Implements the application delegate for the GUI lifecycle and message handling.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self};

use cacao::appkit::menu::{Menu, MenuItem};
use cacao::appkit::window::Window;
use cacao::appkit::{App, AppDelegate};

use crate::gui::view::MarkdownView;
use crate::gui::window::create_main_window;

/// Handles the main window and markdown content updates.
pub struct GuiDelegate {
    window: RefCell<Option<Window>>,
    view: Rc<MarkdownView>,
    receiver: Option<mpsc::Receiver<String>>,
    menu_setup: RefCell<bool>,
}

impl GuiDelegate {
    /// Creates a new GUI delegate with an optional receiver for streamed HTML.
    pub fn new(receiver: Option<mpsc::Receiver<String>>) -> Self {
        GuiDelegate {
            window: RefCell::new(None),
            view: Rc::new(MarkdownView::new()),
            receiver,
            menu_setup: RefCell::new(false),
        }
    }

    /// Set up the main menu for the application
    fn setup_menu(&self) {
        if *self.menu_setup.borrow() {
            return; // Menu already setup
        }

        // Create the main application menu (first menu in macOS menu bar)
        let app_menu_items = vec![
            MenuItem::About("Homo".to_string()), // Standard About menu item
            MenuItem::Separator,
            MenuItem::Quit, // Standard Quit menu item with Cmd+Q
        ];
        let app_menu = Menu::new("Homo", app_menu_items);

        // Create File menu with proper menu items
        let file_menu_items = vec![
            MenuItem::new("New").key("n"),     // Custom item with Cmd+N
            MenuItem::new("Open...").key("o"), // Custom item with Cmd+O
            MenuItem::Separator,
            MenuItem::CloseWindow, // Standard Close Window menu item with Cmd+W
            MenuItem::Separator,
        ];
        let file_menu = Menu::new("File", file_menu_items);

        // Create Window menu with standard menu items
        let window_menu_items = vec![
            MenuItem::Minimize, // Standard Minimize with Cmd+M
            MenuItem::Zoom,     // Standard Zoom
            MenuItem::Separator,
            MenuItem::new("Bring All to Front"),
        ];
        let window_menu = Menu::new("Window", window_menu_items);

        // Set the complete menu bar for the application
        App::set_menu(vec![app_menu, file_menu, window_menu]);

        *self.menu_setup.borrow_mut() = true;
    }
}

impl AppDelegate for GuiDelegate {
    /// Called when the application finishes launching.
    fn did_finish_launching(&self) {
        // Menu setup is now handled when the first window is created
    }

    /// Called periodically; handles window creation and content updates.
    fn did_update(&self) {
        // Check if we have a message receiver (i.e., we're in pipe mode).
        if let Some(receiver) = &self.receiver {
            // Check for a message without blocking.
            if let Ok(html_content) = receiver.try_recv() {
                // If the window doesn't exist yet, this is the first message.
                if self.window.borrow().is_none() {
                    println!("[INFO] First message received. Creating window...");
                    self.setup_menu(); // Set up menu when creating first window
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
