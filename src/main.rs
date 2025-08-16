//! Entry point for the Markdown Viewer application.
//! Handles both GUI and streaming (pipe) modes.

use content::ContentUpdate;
use log::{debug, error, info};
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
    // Initialize logger
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .init();

    debug!("Application starting...");
    let args: Vec<String> = env::args().collect();
    debug!("Command line args: {args:?}");

    // If a filename is provided as an argument, use file mode.
    if args.len() > 1 {
        let filename = &args[1];
        info!("File argument detected: {filename}. Setting up file mode.");
        let (sender, receiver) = mpsc::channel::<ContentUpdate>();
        let filename = filename.clone();
        thread::spawn(move || {
            debug!("File streaming thread started for: {filename}");
            if let Err(e) = streaming::read_from_file(sender, &filename) {
                error!("File streaming thread failed: {e}");
            } else {
                debug!("File streaming thread completed successfully");
            }
        });
        gui::run_app(Some(receiver), false); // File mode
    } else if atty::is(atty::Stream::Stdin) {
        info!(
            "No pipe or file argument detected. Please provide a markdown file as an argument or pipe input. Exiting."
        );
        return Ok(());
    } else {
        info!("Pipe detected. Setting up streaming mode.");
        let (sender, receiver) = mpsc::channel::<ContentUpdate>();
        thread::spawn(move || {
            debug!("Pipe streaming thread started");
            if let Err(e) = streaming::read_from_pipe(sender) {
                error!("Streaming thread failed: {e}");
            } else {
                debug!("Pipe streaming thread completed successfully");
            }
        });
        gui::run_app(Some(receiver), true); // Pipe mode
    }
    debug!("Application exiting");
    Ok(())
}
