//! Streaming logic for reading from stdin and sending HTML updates to the GUI.

use crate::content::{ContentUpdate, DocumentContent};
use crate::error::AppError;
use crate::markdown;
use std::fs::File;
use std::io::{self, Read};
use std::sync::mpsc;

const CHUNK_SIZE: usize = 1024; // Read in 1KB chunks

/// Checks if we're currently inside a code block
fn is_inside_code_block(content: &str) -> bool {
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
        }
    }

    in_code_block
}

/// Checks if we can safely parse at this position (at a block boundary)
fn is_safe_parse_boundary(content: &str) -> bool {
    // Safe boundaries are typically:
    // 1. End of lines (complete lines)
    // 2. After double newlines (paragraph breaks)
    // 3. Not in the middle of code blocks or lists

    if content.is_empty() {
        return false;
    }

    // Never break if we're inside a code block
    if is_inside_code_block(content) {
        return false;
    }

    // Check if content ends with a complete line
    if content.ends_with('\n') {
        // Double newline is always safe (paragraph boundary)
        if content.ends_with("\n\n") {
            return true;
        }

        // Single newline - check if the line before looks complete
        let lines: Vec<&str> = content.lines().collect();
        if let Some(last_line) = lines.last() {
            // Don't break in the middle of potential code blocks
            // Only avoid breaking if it's an opening code block (not closing)
            if last_line.starts_with("```") && !last_line.trim().eq("```") {
                // This is a code block with a language specifier, don't break
                return false;
            }
            // Don't break if we might be in a list continuation
            if last_line.starts_with("   ") || last_line.starts_with("\t") {
                return false;
            }
        }
        return true;
    }

    false
}

/// Finds the last safe boundary position in the content
fn find_last_safe_boundary(content: &str, start_pos: usize) -> Option<usize> {
    let new_content = &content[start_pos..];
    if new_content.is_empty() {
        return None;
    }

    // Look for double newlines first (safest)
    if let Some(pos) = new_content.rfind("\n\n") {
        return Some(start_pos + pos + 2); // Include both newlines
    }

    // Look for single newlines at safe positions
    if let Some(pos) = new_content.rfind('\n') {
        let boundary_content = &content[..start_pos + pos + 1];
        if is_safe_parse_boundary(boundary_content) {
            return Some(start_pos + pos + 1);
        }
    }

    None
}

/// Reads from stdin in chunks, parses accumulated markdown, and sends ContentUpdate to the GUI.
pub fn read_from_pipe(sender: mpsc::Sender<ContentUpdate>) -> Result<(), AppError> {
    let mut stdin = io::stdin();
    let mut buffer = String::new();
    let mut processed_length = 0;
    let mut is_first_chunk = true;

    loop {
        let mut chunk_buf = vec![0; CHUNK_SIZE];
        let bytes_read = stdin.read(&mut chunk_buf)?;

        if bytes_read == 0 {
            // Pipe closed - send any remaining content
            if processed_length < buffer.len() {
                let remaining_content = &buffer[processed_length..];
                if !remaining_content.is_empty() {
                    let html_chunk = markdown::parse_markdown_chunk(
                        remaining_content,
                        &crate::gui::types::ThemeMode::System,
                    );
                    let _ = sender.send(ContentUpdate::Append {
                        markdown: remaining_content.to_string(),
                        html: html_chunk,
                    });
                }
            }
            break;
        }

        chunk_buf.truncate(bytes_read);
        let text_chunk = String::from_utf8_lossy(&chunk_buf);
        buffer.push_str(&text_chunk);

        if is_first_chunk {
            // For first chunk, be more lenient - send content if we have a reasonable amount
            if buffer.len() > 10 {
                if let Some(boundary_pos) = find_last_safe_boundary(&buffer, 0) {
                    let safe_content = &buffer[..boundary_pos];
                    let html_content = markdown::parse_markdown(safe_content);
                    let document_content = DocumentContent::new(
                        safe_content.to_string(),
                        html_content,
                        "Piped Input".to_string(),
                        None,
                    );

                    if sender
                        .send(ContentUpdate::FullReplace(document_content))
                        .is_err()
                    {
                        println!(
                            "[INFO] GUI receiver disconnected. Shutting down streaming thread."
                        );
                        break;
                    }
                    processed_length = boundary_pos;
                    is_first_chunk = false;
                } else if buffer.len() > 100 {
                    // If no safe boundary found but we have substantial content, send it anyway
                    let html_content = markdown::parse_markdown(&buffer);
                    let document_content = DocumentContent::new(
                        buffer.clone(),
                        html_content,
                        "Piped Input".to_string(),
                        None,
                    );

                    if sender
                        .send(ContentUpdate::FullReplace(document_content))
                        .is_err()
                    {
                        println!(
                            "[INFO] GUI receiver disconnected. Shutting down streaming thread."
                        );
                        break;
                    }
                    processed_length = buffer.len();
                    is_first_chunk = false;
                }
            }
        } else {
            // For subsequent chunks, look for safe boundaries to send incremental updates
            if let Some(boundary_pos) = find_last_safe_boundary(&buffer, processed_length) {
                let new_content = &buffer[processed_length..boundary_pos];
                if !new_content.is_empty() {
                    let html_chunk = markdown::parse_markdown_chunk(
                        new_content,
                        &crate::gui::types::ThemeMode::System,
                    );

                    if sender
                        .send(ContentUpdate::Append {
                            markdown: new_content.to_string(),
                            html: html_chunk,
                        })
                        .is_err()
                    {
                        println!(
                            "[INFO] GUI receiver disconnected. Shutting down streaming thread."
                        );
                        break;
                    }
                    processed_length = boundary_pos;
                }
            }
        }
    }

    Ok(())
}

/// Reads the entire file, parses markdown, and sends ContentUpdate to the GUI.
pub fn read_from_file(sender: mpsc::Sender<ContentUpdate>, filename: &str) -> Result<(), AppError> {
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
    let _ = sender.send(ContentUpdate::FullReplace(document_content));
    Ok(())
}
