use std::sync::mpsc::SendError;
use thiserror::Error;

/// The unified error type for the entire application.
#[derive(Debug, Error)]
pub enum AppError {
    /// Represents all errors that can occur during I/O operations.
    /// The `#[from]` attribute automatically converts a `std::io::Error`
    /// into an `AppError::Io`.
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    /// Represents an error that can occur when sending a message
    /// from the streaming thread to the GUI thread. This happens if the
    /// GUI has already closed and the channel is broken.
    #[error("Channel Send Error: {0}")]
    ChannelSend(#[from] SendError<String>),
}
