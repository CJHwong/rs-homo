//! Streaming logic for reading from stdin and sending HTML updates to the GUI.

use crate::error::AppError;
use crate::markdown;
use std::io::{self, Read};
use std::sync::mpsc;

const CHUNK_SIZE: usize = 1024; // Read in 1KB chunks

/// Reads from stdin in chunks, parses accumulated markdown, and sends HTML to the GUI.
pub fn read_from_pipe(sender: mpsc::Sender<String>) -> Result<(), AppError> {
    let mut stdin = io::stdin();
    let mut buffer = String::new();

    loop {
        let mut chunk_buf = vec![0; CHUNK_SIZE];
        let bytes_read = stdin.read(&mut chunk_buf)?;

        if bytes_read == 0 {
            break; // Pipe closed
        }

        chunk_buf.truncate(bytes_read);
        let text_chunk = String::from_utf8_lossy(&chunk_buf);
        buffer.push_str(&text_chunk);

        // Parse the entire buffer and send HTML to the GUI.
        let html_content = markdown::parse_markdown(&buffer);

        // If the receiver is disconnected, exit the thread gracefully.
        if sender.send(html_content).is_err() {
            println!("[INFO] GUI receiver disconnected. Shutting down streaming thread.");
            break;
        }
    }

    Ok(())
}
