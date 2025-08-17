//! Streaming logic for reading from stdin and sending HTML updates to the GUI.

use crate::content::{ContentUpdate, DocumentContent};
use crate::error::AppError;
use crate::markdown;
use log::{debug, error, info};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::sync::mpsc;

/// Tracks the state of markdown parsing during streaming
#[derive(Debug, Clone)]
struct StreamingState {
    /// Whether we're currently inside a code block
    in_code_block: bool,
    /// The language of the current code block (if any)
    code_language: String,
    /// Accumulated markdown content
    markdown_buffer: String,
    /// Track if we've sent the first content update
    sent_first_update: bool,
    /// Lines accumulated since last update
    lines_since_update: usize,
}

impl StreamingState {
    fn new() -> Self {
        Self {
            in_code_block: false,
            code_language: String::new(),
            markdown_buffer: String::new(),
            sent_first_update: false,
            lines_since_update: 0,
        }
    }

    /// Processes a line and returns whether we should send an update
    fn process_line(&mut self, line: &str) -> bool {
        self.lines_since_update += 1;
        self.markdown_buffer.push_str(line);
        self.markdown_buffer.push('\n');

        let trimmed = line.trim();

        // Check for code block start/end
        if trimmed.starts_with("```") {
            if !self.in_code_block {
                // Starting a code block
                self.in_code_block = true;
                self.code_language = trimmed.strip_prefix("```").unwrap_or("").to_string();
                debug!(
                    "Starting code block with language: '{}'",
                    self.code_language
                );
            } else {
                // Ending a code block
                self.in_code_block = false;
                self.code_language.clear();
                debug!("Ending code block");
                // Always send update after code block ends
                return true;
            }
        }

        // Send update conditions (increased thresholds for better rapid streaming performance):
        // IMPORTANT: Never send updates while inside a code block to prevent splitting
        if !self.in_code_block {
            // 1. First substantial content (after 5 lines, was 3)
            if !self.sent_first_update && self.lines_since_update >= 5 {
                return true;
            }

            // 2. Send update after paragraph breaks (empty lines) with more accumulation
            if trimmed.is_empty() && self.lines_since_update >= 5 {
                return true;
            }

            // 3. Send update after accumulating more lines to reduce rapid updates
            if self.lines_since_update >= 10 {
                return true;
            }
        }

        false
    }

    /// Marks that an update was sent and resets counters
    fn mark_update_sent(&mut self) {
        self.sent_first_update = true;
        self.lines_since_update = 0;
    }

    /// Gets the current markdown content
    fn get_content(&self) -> &str {
        &self.markdown_buffer
    }

    /// Clears the buffer (for full replace updates)
    fn clear_buffer(&mut self) {
        self.markdown_buffer.clear();
    }
}

/// Reads from stdin line-by-line using state machine, sending incremental updates to the GUI.
pub fn read_from_pipe_stateful(sender: mpsc::Sender<ContentUpdate>) -> Result<(), AppError> {
    debug!("Starting stateful line-by-line reading from stdin");
    let stdin = io::stdin();
    let reader = BufReader::new(stdin);
    let mut state = StreamingState::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(line) => line,
            Err(e) => {
                error!("Failed to read line {}: {}", line_num + 1, e);
                return Err(AppError::from(e));
            }
        };

        debug!("Processing line {}: {:?}", line_num + 1, line);

        // Process the line and check if we should send an update
        let should_update = state.process_line(&line);

        if should_update {
            let content = state.get_content().to_string();
            debug!(
                "Sending update with {} bytes after line {}",
                content.len(),
                line_num + 1
            );

            // Parse just the new content chunk
            let html_content = markdown::parse_markdown(&content);

            let update = if state.sent_first_update {
                // For subsequent updates, use Append with just the new content
                ContentUpdate::Append {
                    markdown: content,
                    html: html_content,
                }
            } else {
                // First update: use FullReplace to establish initial content
                let document_content =
                    DocumentContent::new(content, html_content, "Piped Input".to_string(), None);
                ContentUpdate::FullReplace(document_content)
            };

            match sender.send(update) {
                Ok(()) => {
                    debug!(
                        "Successfully sent content update after line {}",
                        line_num + 1
                    );
                    state.mark_update_sent();
                    state.clear_buffer(); // Clear buffer after successful send
                }
                Err(e) => {
                    error!("Failed to send content update: {e}");
                    info!("GUI receiver disconnected. Shutting down streaming thread.");
                    break;
                }
            }
        }
    }

    // Send any remaining content
    if !state.get_content().is_empty() {
        let content = state.get_content().to_string();
        let html_content = markdown::parse_markdown(&content);

        let update = if state.sent_first_update {
            ContentUpdate::Append {
                markdown: content,
                html: html_content,
            }
        } else {
            // Final content is also the first content
            let document_content =
                DocumentContent::new(content, html_content, "Piped Input".to_string(), None);
            ContentUpdate::FullReplace(document_content)
        };

        match sender.send(update) {
            Ok(()) => debug!("Successfully sent final content update"),
            Err(e) => error!("Failed to send final content: {e}"),
        }
    }

    debug!("Finished reading from stdin");
    Ok(())
}

/// Main entry point for reading from stdin pipes.
/// Uses the new stateful line-by-line approach.
pub fn read_from_pipe(sender: mpsc::Sender<ContentUpdate>) -> Result<(), AppError> {
    read_from_pipe_stateful(sender)
}

/// Reads the entire file, parses markdown, and sends ContentUpdate to the GUI.
pub fn read_from_file(sender: mpsc::Sender<ContentUpdate>, filename: &str) -> Result<(), AppError> {
    debug!("Opening file: {filename}");
    let mut file = File::open(filename)?;
    let mut buffer = String::new();

    debug!("Reading file content");
    file.read_to_string(&mut buffer)?;
    let buffer_len = buffer.len();
    debug!("Read {buffer_len} bytes from file");

    debug!("Parsing markdown");
    let html_content = markdown::parse_markdown(&buffer);
    let title = std::path::Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string();
    debug!("File title: {title}");

    let document_content =
        DocumentContent::new(buffer, html_content, title, Some(filename.to_string()));

    debug!("Sending content update to GUI");
    match sender.send(ContentUpdate::FullReplace(document_content)) {
        Ok(()) => debug!("Successfully sent file content to GUI"),
        Err(e) => error!("Failed to send content to GUI: {e}"),
    }
    Ok(())
}
