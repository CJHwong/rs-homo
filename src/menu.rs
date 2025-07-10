use cacao::appkit::menu::{Menu, MenuItem};
use std::sync::mpsc;

pub enum MenuMessage {
    ToggleMode,
    Copy,
    SelectAll,
}

static mut MENU_SENDER: Option<mpsc::Sender<MenuMessage>> = None;

pub fn set_menu_sender(sender: mpsc::Sender<MenuMessage>) {
    unsafe {
        MENU_SENDER = Some(sender);
    }
}

pub fn dispatch_menu_message(message: MenuMessage) {
    unsafe {
        if let Some(ref sender) = MENU_SENDER {
            let _ = sender.send(message);
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
            vec![MenuItem::new("Toggle Mode").key("t").action(|| {
                dispatch_menu_message(MenuMessage::ToggleMode);
            })],
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
