use cacao::appkit::menu::{Menu, MenuItem};
use log::{debug, error};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use crate::gui::types::{FontFamily, ThemeMode};

#[derive(Debug)]
pub enum MenuMessage {
    ToggleMode,
    Copy,
    SelectAll,
    SetFontFamily(FontFamily),
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    SetTheme(ThemeMode),
}

use std::sync::LazyLock;

static MENU_SENDER: LazyLock<Arc<Mutex<Option<mpsc::Sender<MenuMessage>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

pub fn set_menu_sender(sender: mpsc::Sender<MenuMessage>) {
    if let Ok(mut sender_guard) = MENU_SENDER.lock() {
        *sender_guard = Some(sender);
        debug!("Menu sender set successfully");
    } else {
        error!("Failed to lock MENU_SENDER for setting");
    }
}

pub fn dispatch_menu_message(message: MenuMessage) {
    match MENU_SENDER.lock() {
        Ok(sender_guard) => {
            if let Some(ref sender) = *sender_guard {
                debug!("Dispatching menu message: {message:?}");
                match sender.send(message) {
                    Ok(_) => debug!("Message sent successfully"),
                    Err(e) => error!("Failed to send message: {e:?}"),
                }
            } else {
                error!("MENU_SENDER is None!");
            }
        }
        Err(e) => {
            error!("Failed to lock MENU_SENDER: {e:?}");
        }
    }
}

pub fn create_menus() -> Vec<Menu> {
    vec![
        // App menu
        Menu::new(
            "Homo",
            vec![
                MenuItem::About("Homo".to_string()),
                MenuItem::Separator,
                MenuItem::Quit,
            ],
        ),
        // File menu
        Menu::new(
            "File",
            vec![
                MenuItem::new("New").key("n"),
                MenuItem::new("Open...").key("o"),
                MenuItem::Separator,
                MenuItem::CloseWindow,
            ],
        ),
        // Edit menu
        Menu::new(
            "Edit",
            vec![
                MenuItem::new("Copy").key("c").action(|| {
                    dispatch_menu_message(MenuMessage::Copy);
                }),
                MenuItem::Separator,
                MenuItem::new("Select All").key("a").action(|| {
                    dispatch_menu_message(MenuMessage::SelectAll);
                }),
            ],
        ),
        // View menu
        Menu::new(
            "View",
            vec![
                MenuItem::new("Toggle Mode").key("t").action(|| {
                    dispatch_menu_message(MenuMessage::ToggleMode);
                }),
                MenuItem::Separator,
                MenuItem::new("System Font").key("1").action(|| {
                    dispatch_menu_message(MenuMessage::SetFontFamily(FontFamily::System));
                }),
                MenuItem::new("Menlo Font").key("2").action(|| {
                    dispatch_menu_message(MenuMessage::SetFontFamily(FontFamily::Menlo));
                }),
                MenuItem::new("Monaco Font").key("3").action(|| {
                    dispatch_menu_message(MenuMessage::SetFontFamily(FontFamily::Monaco));
                }),
                MenuItem::new("Helvetica Font").key("4").action(|| {
                    dispatch_menu_message(MenuMessage::SetFontFamily(FontFamily::Helvetica));
                }),
                MenuItem::Separator,
                MenuItem::new("Light Theme").key("l").action(|| {
                    dispatch_menu_message(MenuMessage::SetTheme(ThemeMode::Light));
                }),
                MenuItem::new("Dark Theme").key("d").action(|| {
                    dispatch_menu_message(MenuMessage::SetTheme(ThemeMode::Dark));
                }),
                MenuItem::new("System Theme").key("s").action(|| {
                    dispatch_menu_message(MenuMessage::SetTheme(ThemeMode::System));
                }),
                MenuItem::Separator,
                MenuItem::new("Increase Font Size").key("=").action(|| {
                    dispatch_menu_message(MenuMessage::IncreaseFontSize);
                }),
                MenuItem::new("Decrease Font Size").key("-").action(|| {
                    dispatch_menu_message(MenuMessage::DecreaseFontSize);
                }),
                MenuItem::new("Reset Font Size").key("0").action(|| {
                    dispatch_menu_message(MenuMessage::ResetFontSize);
                }),
            ],
        ),
        // Window menu
        Menu::new(
            "Window",
            vec![
                MenuItem::Minimize,
                MenuItem::Zoom,
                MenuItem::Separator,
                MenuItem::new("Bring All to Front"),
            ],
        ),
    ]
}
