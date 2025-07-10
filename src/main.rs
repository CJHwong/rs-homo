//! Entry point for the Markdown Viewer application.
//! Handles both GUI and streaming (pipe) modes.

use content::DocumentContent;
use std::env;
use std::sync::mpsc;
use std::thread;

mod content;
mod error;
mod gui;
mod markdown;
mod menu;
mod streaming;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    // If a filename is provided as an argument, use file mode.
    if args.len() > 1 {
        let filename = &args[1];
        println!("File argument detected: {filename}. Setting up file mode.");
        let (sender, receiver) = mpsc::channel::<DocumentContent>();
        let filename = filename.clone();
        thread::spawn(move || {
            if let Err(e) = streaming::read_from_file(sender, &filename) {
                eprintln!("[ERROR] File streaming thread failed: {e}");
            }
        });
        gui::run_app(Some(receiver), false); // File mode
    } else if atty::is(atty::Stream::Stdin) {
        println!(
            "No pipe or file argument detected. Please provide a markdown file as an argument or pipe input. Exiting."
        );
        return Ok(());
    } else {
        println!("Pipe detected. Setting up streaming mode.");
        let (sender, receiver) = mpsc::channel::<DocumentContent>();
        thread::spawn(move || {
            if let Err(e) = streaming::read_from_pipe(sender) {
                eprintln!("[ERROR] Streaming thread failed: {e}");
            }
        });
        gui::run_app(Some(receiver), true); // Pipe mode
    }
    Ok(())
}
