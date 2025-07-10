//! Streaming logic for reading from stdin and sending HTML updates to the GUI.

use crate::content::DocumentContent;
use crate::error::AppError;
use crate::markdown;
use std::fs::File;
use std::io::{self, Read};
use std::sync::mpsc;

const CHUNK_SIZE: usize = 1024; // Read in 1KB chunks

/// Reads from stdin in chunks, parses accumulated markdown, and sends DocumentContent to the GUI.
pub fn read_from_pipe(sender: mpsc::Sender<DocumentContent>) -> Result<(), AppError> {
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

        // Parse the entire buffer and create DocumentContent
        let html_content = markdown::parse_markdown(&buffer);
        let document_content = DocumentContent::new(
            buffer.clone(),
            html_content,
            "Piped Input".to_string(),
            None,
        );

        // If the receiver is disconnected, exit the thread gracefully.
        if sender.send(document_content).is_err() {
            println!("[INFO] GUI receiver disconnected. Shutting down streaming thread.");
            break;
        }
    }

    Ok(())
}

/// Reads the entire file, parses markdown, and sends DocumentContent to the GUI.
pub fn read_from_file(
    sender: mpsc::Sender<DocumentContent>,
    filename: &str,
) -> Result<(), AppError> {
    let mut file = File::open(filename)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    let html_content = markdown::parse_markdown(&buffer);
    let title = std::path::Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let document_content =
        DocumentContent::new(buffer, html_content, title, Some(filename.to_string()));

    // If the receiver is disconnected, exit gracefully.
    let _ = sender.send(document_content);
    Ok(())
}
