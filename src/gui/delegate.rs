//! Implements the application delegate for the GUI lifecycle and message handling.

#![allow(unexpected_cfgs)] // Suppress objc crate cfg warnings
#![allow(deprecated)] // Suppress cocoa crate deprecation warnings until objc2 ecosystem is mature

use std::cell::RefCell;
use std::collections::VecDeque;
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
    pending_content: Arc<Mutex<VecDeque<ContentUpdate>>>,
    style_preferences: RefCell<StylePreferences>,
    last_update_time: RefCell<std::time::Instant>,
    pending_batch: RefCell<Vec<ContentUpdate>>,
    // Rate detection and adaptive processing
    update_timestamps: RefCell<VecDeque<std::time::Instant>>,
    current_rate_category: RefCell<InputRateCategory>,
}

#[derive(Debug, Clone, PartialEq)]
enum InputRateCategory {
    Slow,    // > 0.1s between updates (use incremental appends)
    Medium,  // 0.01s - 0.1s (use batching with 200ms windows)
    Fast,    // 0.001s - 0.01s (use aggressive batching with 500ms windows)
    Extreme, // < 0.001s (use full reload strategy)
}

impl GuiDelegate {
    /// Creates a new GUI delegate with an optional receiver for streamed ContentUpdate.
    pub fn new(receiver: Option<mpsc::Receiver<ContentUpdate>>, is_pipe_mode: bool) -> Self {
        // Set up menu message channel
        let (menu_sender, menu_receiver) = mpsc::channel();
        menu::set_menu_sender(menu_sender);

        // Create shared state for pending content queue
        let pending_content = Arc::new(Mutex::new(VecDeque::new()));

        // Start background thread to continuously poll original receiver
        if let Some(orig_receiver) = receiver {
            let pending_content_clone = pending_content.clone();
            thread::spawn(move || {
                while let Ok(content_update) = orig_receiver.recv() {
                    if let Ok(mut pending) = pending_content_clone.lock() {
                        pending.push_back(content_update);
                        debug!("Queued content update, queue size: {}", pending.len());
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
            last_update_time: RefCell::new(std::time::Instant::now()),
            pending_batch: RefCell::new(Vec::new()),
            update_timestamps: RefCell::new(VecDeque::new()),
            current_rate_category: RefCell::new(InputRateCategory::Slow),
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

    /// Detect input rate and update processing strategy
    fn detect_and_update_rate_category(&self) {
        let now = std::time::Instant::now();
        let mut timestamps = self.update_timestamps.borrow_mut();

        // Add current timestamp
        timestamps.push_back(now);

        // Keep only recent timestamps (last 10 updates or 2 seconds)
        while timestamps.len() > 10 {
            timestamps.pop_front();
        }
        while let Some(&front) = timestamps.front() {
            if now.duration_since(front) > Duration::from_secs(2) {
                timestamps.pop_front();
            } else {
                break;
            }
        }

        // Calculate average interval if we have enough samples
        if timestamps.len() >= 3 {
            let total_duration = now.duration_since(*timestamps.front().unwrap());
            let avg_interval = total_duration / (timestamps.len() as u32 - 1);

            let new_category = if avg_interval > Duration::from_millis(100) {
                InputRateCategory::Slow
            } else if avg_interval > Duration::from_millis(10) {
                InputRateCategory::Medium
            } else if avg_interval > Duration::from_millis(1) {
                InputRateCategory::Fast
            } else {
                InputRateCategory::Extreme
            };

            let mut current_category = self.current_rate_category.borrow_mut();
            if *current_category != new_category {
                debug!(
                    "Input rate changed from {:?} to {:?} (avg interval: {:?})",
                    *current_category, new_category, avg_interval
                );
                *current_category = new_category;
            }
        }
    }

    /// Get adaptive processing window based on input rate
    fn get_processing_window(&self) -> Duration {
        match *self.current_rate_category.borrow() {
            InputRateCategory::Slow => Duration::from_millis(50), // Process quickly for slow input
            InputRateCategory::Medium => Duration::from_millis(200), // Moderate batching
            InputRateCategory::Fast => Duration::from_millis(500), // Aggressive batching
            InputRateCategory::Extreme => Duration::from_millis(1000), // Very aggressive batching
        }
    }

    /// Set up background polling that properly dispatches to main thread  
    fn start_background_polling(&self) {
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_millis(100));

                // Use performSelectorOnMainThread to safely call updateWindows from background thread
                // SAFETY: performSelectorOnMainThread is designed for cross-thread communication
                unsafe {
                    use cocoa::appkit::NSApp;
                    use cocoa::base::{NO, id, nil};
                    use core_foundation::runloop::{CFRunLoopGetMain, CFRunLoopWakeUp};
                    use objc::{msg_send, sel, sel_impl};

                    let app: id = NSApp();
                    if app != nil {
                        // Use performSelectorOnMainThread to safely execute on main thread
                        let _: () = msg_send![app,  performSelectorOnMainThread:sel!(updateWindows) withObject:nil waitUntilDone:NO];
                    }

                    // Also wake up the main run loop
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

        // Adaptive content processing based on input rate
        let now = std::time::Instant::now();
        let mut last_update = self.last_update_time.borrow_mut();
        let time_since_last_update = now.duration_since(*last_update);

        // Collect updates from the queue and detect rate
        let mut updates_to_process = Vec::new();
        let mut has_new_updates = false;

        while let Ok(mut pending) = self.pending_content.lock() {
            if let Some(content_update) = pending.pop_front() {
                updates_to_process.push(content_update);
                has_new_updates = true;
                // Detect input rate when we get new updates
                self.detect_and_update_rate_category();

                // Limit batch size based on rate category
                let max_batch_size = match *self.current_rate_category.borrow() {
                    InputRateCategory::Slow => 5,
                    InputRateCategory::Medium => 15,
                    InputRateCategory::Fast => 50,
                    InputRateCategory::Extreme => 200,
                };

                if updates_to_process.len() >= max_batch_size {
                    break;
                }
            } else {
                break;
            }
        }

        // Add any new updates to the pending batch
        if has_new_updates {
            self.pending_batch.borrow_mut().extend(updates_to_process);
        }

        // Get adaptive processing window
        let processing_window = self.get_processing_window();

        // Decide whether to process based on adaptive timing and conditions
        let should_process = time_since_last_update >= processing_window
            || self
                .pending_batch
                .borrow()
                .iter()
                .any(|update| matches!(update, ContentUpdate::FullReplace(_)))
            || (matches!(
                *self.current_rate_category.borrow(),
                InputRateCategory::Extreme
            ) && self.pending_batch.borrow().len() > 100);

        if should_process && !self.pending_batch.borrow().is_empty() {
            let batched_updates = std::mem::take(&mut *self.pending_batch.borrow_mut());
            let rate_category = self.current_rate_category.borrow().clone();

            debug!(
                "Processing batch of {} updates (rate: {:?}, window: {:?})",
                batched_updates.len(),
                rate_category,
                processing_window
            );

            // Use different strategies based on input rate
            match rate_category {
                InputRateCategory::Slow | InputRateCategory::Medium => {
                    // Normal incremental processing for manageable rates
                    self.process_updates_incrementally(batched_updates);
                }
                InputRateCategory::Fast | InputRateCategory::Extreme => {
                    // Aggressive batching or full reload for high rates
                    self.process_updates_aggressively(batched_updates);
                }
            }

            *last_update = now;
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

impl GuiDelegate {
    /// Process updates incrementally for slow/medium input rates
    fn process_updates_incrementally(&self, batched_updates: Vec<ContentUpdate>) {
        // Combine consecutive Append updates into a single update for efficiency
        let mut combined_updates = Vec::new();
        let mut current_markdown = String::new();
        let mut current_html = String::new();

        for update in batched_updates {
            match update {
                ContentUpdate::FullReplace(content) => {
                    // Flush any pending appends before the full replace
                    if !current_markdown.is_empty() {
                        combined_updates.push(ContentUpdate::Append {
                            markdown: current_markdown.clone(),
                            html: current_html.clone(),
                        });
                        current_markdown.clear();
                        current_html.clear();
                    }
                    combined_updates.push(ContentUpdate::FullReplace(content));
                }
                ContentUpdate::Append { markdown, html } => {
                    current_markdown.push_str(&markdown);
                    current_html.push_str(&html);
                }
            }
        }

        // Add any remaining appends
        if !current_markdown.is_empty() {
            combined_updates.push(ContentUpdate::Append {
                markdown: current_markdown,
                html: current_html,
            });
        }

        // Process the combined updates normally
        for update in combined_updates {
            self.process_content_update(update);
        }
    }

    /// Process updates aggressively for fast/extreme input rates
    fn process_updates_aggressively(&self, batched_updates: Vec<ContentUpdate>) {
        // For high-speed input, skip incremental appends and do full rebuilds
        let mut final_markdown = String::new();
        let mut found_full_replace = false;
        let mut base_content: Option<DocumentContent> = None;

        // Accumulate all content changes
        for update in batched_updates {
            match update {
                ContentUpdate::FullReplace(content) => {
                    base_content = Some(content);
                    found_full_replace = true;
                    final_markdown.clear(); // Reset on full replace
                }
                ContentUpdate::Append { markdown, .. } => {
                    final_markdown.push_str(&markdown);
                }
            }
        }

        if found_full_replace {
            // We have a base document, append all accumulated content
            if let Some(mut content) = base_content {
                content.markdown.push_str(&final_markdown);
                content.regenerate_html();

                debug!(
                    "Aggressive processing: full reload with {} total chars",
                    content.markdown.len()
                );
                self.process_content_update(ContentUpdate::FullReplace(content));
            }
        } else if !final_markdown.is_empty() {
            // Only appends, update the current document directly
            if let Some(ref mut current_doc) = *self.current_document.borrow_mut() {
                current_doc.markdown.push_str(&final_markdown);
                current_doc.regenerate_html();

                // Force a full reload instead of incremental append for extreme speeds
                debug!("Aggressive processing: forced full reload with accumulated content");
                self.view
                    .update_content_with_scroll(current_doc, ScrollBehavior::Bottom);
            }
        }
    }

    /// Process a single content update
    fn process_content_update(&self, content_update: ContentUpdate) {
        match content_update {
            ContentUpdate::FullReplace(mut content) => {
                // Apply current style preferences to the content
                content.style_preferences = self.style_preferences.borrow().clone();

                // Create window if needed
                if self.window.borrow().is_none() {
                    info!("First message received. Creating window...");
                    self.setup_menu();
                    let window =
                        create_main_window_with_content(&self.view, &content, self.is_pipe_mode);
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

                    // Update the current document with the new content
                    if let Some(ref mut current_doc) = *self.current_document.borrow_mut() {
                        debug!(
                            "Before append - current doc markdown length: {}",
                            current_doc.markdown.len()
                        );
                        current_doc.markdown.push_str(&markdown);
                        debug!(
                            "After append - current doc markdown length: {}",
                            current_doc.markdown.len()
                        );
                        debug!(
                            "First 200 chars of accumulated markdown: {:?}",
                            current_doc.markdown.chars().take(200).collect::<String>()
                        );

                        // Regenerate HTML to ensure consistency with accumulated content
                        current_doc.regenerate_html();
                        debug!(
                            "After regenerate - current doc HTML length: {}",
                            current_doc.html.len()
                        );

                        // Try to append the individual chunk first
                        self.view
                            .append_content(&markdown, &html, &style_preferences);
                        debug!("Content appended (chunk: {} bytes)", markdown.len());
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
}
