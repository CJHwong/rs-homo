//! Entry point for the Markdown Viewer application.
//! Handles both GUI and streaming (pipe) modes.

use std::sync::mpsc;
use std::thread;

mod error;
mod gui;
mod markdown;
mod streaming;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If stdin is a TTY, launch the GUI without streaming.
    if atty::is(atty::Stream::Stdin) {
        println!("No pipe detected. Opening an empty window.");
        gui::run_app(None);
    } else {
        println!("Pipe detected. Setting up streaming mode.");
        // Set up a channel for streaming markdown updates from stdin to the GUI.
        let (sender, receiver) = mpsc::channel::<String>();
        thread::spawn(move || {
            if let Err(e) = streaming::read_from_pipe(sender) {
                eprintln!("[ERROR] Streaming thread failed: {e}");
            }
        });
        gui::run_app(Some(receiver));
    }
    Ok(())
}
