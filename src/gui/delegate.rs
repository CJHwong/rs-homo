//! Implements the application delegate for the GUI lifecycle and message handling.

#![allow(unexpected_cfgs)] // Suppress objc crate cfg warnings
#![allow(deprecated)] // Suppress cocoa crate deprecation warnings until objc2 ecosystem is mature

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cacao::appkit::window::Window;
use cacao::appkit::{App, AppDelegate};
use log::{debug, info};

use crate::content::{ContentUpdate, DocumentContent};
use crate::gui::types::{FontFamily, StylePreferences, ThemeMode};
use crate::gui::view::{MarkdownView, ScrollBehavior};
use crate::gui::window::{create_main_window, create_main_window_with_content};
use crate::menu::{self, MenuMessage};

/// Handles the main window and markdown content updates.
pub struct GuiDelegate {
    window: RefCell<Option<Window>>,
    view: Rc<MarkdownView>,
    menu_setup: RefCell<bool>,
    current_document: RefCell<Option<DocumentContent>>,
    menu_receiver: RefCell<Option<mpsc::Receiver<MenuMessage>>>,
    is_pipe_mode: bool,
    pending_content: Arc<Mutex<Option<ContentUpdate>>>,
    style_preferences: RefCell<StylePreferences>,
}

impl GuiDelegate {
    /// Creates a new GUI delegate with an optional receiver for streamed ContentUpdate.
    pub fn new(receiver: Option<mpsc::Receiver<ContentUpdate>>, is_pipe_mode: bool) -> Self {
        // Set up menu message channel
        let (menu_sender, menu_receiver) = mpsc::channel();
        menu::set_menu_sender(menu_sender);

        // Create shared state for pending content
        let pending_content = Arc::new(Mutex::new(None));

        // Start background thread to continuously poll original receiver
        if let Some(orig_receiver) = receiver {
            let pending_content_clone = pending_content.clone();
            thread::spawn(move || {
                while let Ok(content_update) = orig_receiver.recv() {
                    if let Ok(mut pending) = pending_content_clone.lock() {
                        *pending = Some(content_update);
                    }
                }
            });
        }

        GuiDelegate {
            window: RefCell::new(None),
            view: Rc::new(MarkdownView::new()),
            menu_setup: RefCell::new(false),
            current_document: RefCell::new(None),
            menu_receiver: RefCell::new(Some(menu_receiver)),
            is_pipe_mode,
            pending_content,
            style_preferences: RefCell::new(StylePreferences::load_from_user_defaults()),
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
        let style_preferences = self.style_preferences.borrow().clone();
        self.view.toggle_mode(&style_preferences);
    }

    /// Handles font family change
    pub fn set_font_family(&self, font_family: FontFamily) {
        self.style_preferences.borrow_mut().font_family = font_family;
        self.style_preferences.borrow().save_to_user_defaults();
        self.update_content_with_new_styles();
    }

    /// Increases font size
    pub fn increase_font_size(&self) {
        self.style_preferences.borrow_mut().increase_font_size();
        self.style_preferences.borrow().save_to_user_defaults();
        self.update_content_with_new_styles();
    }

    /// Decreases font size
    pub fn decrease_font_size(&self) {
        self.style_preferences.borrow_mut().decrease_font_size();
        self.style_preferences.borrow().save_to_user_defaults();
        self.update_content_with_new_styles();
    }

    /// Resets font size to default
    pub fn reset_font_size(&self) {
        self.style_preferences.borrow_mut().reset_font_size();
        self.style_preferences.borrow().save_to_user_defaults();
        self.update_content_with_new_styles();
    }

    /// Handles theme change
    pub fn set_theme(&self, theme: ThemeMode) {
        self.style_preferences.borrow_mut().theme = theme;
        self.style_preferences.borrow().save_to_user_defaults();
        self.update_content_with_new_styles();
    }

    /// Updates the content with new styling preferences
    fn update_content_with_new_styles(&self) {
        let mut current_document_option = self.current_document.borrow_mut();
        if let Some(current_document) = current_document_option.as_mut() {
            current_document.style_preferences = self.style_preferences.borrow().clone();
            // Regenerate HTML with new theme for syntax highlighting
            current_document.regenerate_html();
            self.view.update_content(current_document);
        }
    }

    /// Force the event loop to stay active by posting periodic events
    fn start_background_polling(&self) {
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_millis(100));

                // Force the event loop to stay active
                // SAFETY: All unsafe operations here are necessary for forcing macOS event loop activity:
                // 1. NSApp() - Required to get NSApplication instance (Objective-C runtime call)
                // 2. msg_send! - Required for Objective-C method dispatch to updateWindows
                // 3. CFRunLoop* - Required C FFI calls to wake up the main run loop
                unsafe {
                    use cocoa::appkit::NSApp;
                    use cocoa::base::{id, nil};
                    use core_foundation::runloop::{CFRunLoopGetMain, CFRunLoopWakeUp};
                    use objc::{msg_send, sel, sel_impl};

                    // Get NSApplication shared instance
                    let app: id = NSApp();

                    if app != nil {
                        // Force app to process events by calling updateWindows
                        let _: () = msg_send![app, updateWindows];
                    }

                    // Wake up the main run loop
                    let main_loop = CFRunLoopGetMain();
                    CFRunLoopWakeUp(main_loop);
                }
            }
        });
    }
}

impl AppDelegate for GuiDelegate {
    /// Called when the application finishes launching.
    fn did_finish_launching(&self) {
        // Menu setup is now handled when the first window is created
        // Set up background polling to ensure updates continue when window is not focused
        self.start_background_polling();
    }

    /// Called when forced by background thread - handles all updates
    fn did_update(&self) {
        // Handle menu messages
        if let Some(menu_receiver) = self.menu_receiver.borrow().as_ref() {
            while let Ok(menu_message) = menu_receiver.try_recv() {
                debug!("Received menu message: {menu_message:?}");
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
                    MenuMessage::SetFontFamily(font_family) => {
                        self.set_font_family(font_family);
                    }
                    MenuMessage::IncreaseFontSize => {
                        self.increase_font_size();
                    }
                    MenuMessage::DecreaseFontSize => {
                        self.decrease_font_size();
                    }
                    MenuMessage::ResetFontSize => {
                        self.reset_font_size();
                    }
                    MenuMessage::SetTheme(theme) => {
                        self.set_theme(theme);
                    }
                }
            }
        }

        // Check for pending content and update
        if let Ok(mut pending) = self.pending_content.lock()
            && let Some(content_update) = pending.take()
        {
            match content_update {
                ContentUpdate::FullReplace(mut content) => {
                    // Apply current style preferences to the content
                    content.style_preferences = self.style_preferences.borrow().clone();

                    // Create window if needed
                    if self.window.borrow().is_none() {
                        info!("First message received. Creating window...");
                        self.setup_menu();
                        let window = create_main_window_with_content(
                            &self.view,
                            &content,
                            self.is_pipe_mode,
                        );
                        *self.window.borrow_mut() = Some(window);
                    }

                    // Update content
                    let scroll_behavior = if self.is_pipe_mode {
                        ScrollBehavior::Bottom
                    } else {
                        ScrollBehavior::Top
                    };

                    self.view
                        .update_content_with_scroll(&content, scroll_behavior);
                    *self.current_document.borrow_mut() = Some(content);
                    debug!("Content updated (full replace)");
                }
                ContentUpdate::Append { markdown, html } => {
                    // Only append if we have a window
                    if self.window.borrow().is_some() {
                        let style_preferences = self.style_preferences.borrow().clone();
                        self.view
                            .append_content(&markdown, &html, &style_preferences);
                        debug!("Content appended");
                    }
                }
            }
        }

        // Create empty window if needed
        if self.window.borrow().is_none() {
            info!("Creating empty window...");
            self.setup_menu();
            let window = create_main_window(&self.view);
            *self.window.borrow_mut() = Some(window);
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
