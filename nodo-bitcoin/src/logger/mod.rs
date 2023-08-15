use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use crate::{constants::PATH_LOG, node_error::NodeError};

/// A logger that writes messages to a file.
#[derive(Clone)]
pub struct Logger {
    sender: Sender<String>,
}

impl Logger {
    /// Creates a new `Logger` instance using the specified log file path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the log file.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if the log file could not be opened.
    fn new_from_path(path: &str) -> Result<Logger, NodeError> {
        let (sender, receiver) = mpsc::channel();
        Logger::start(receiver, path)?;
        Ok(Logger { sender })
    }

    /// Creates a new `Logger` instance using the `PATH_LOG` environment variable
    /// as the log file path.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::EnvironVarNotFound` if the `PATH_LOG` environment variable is not set.
    pub fn new() -> Result<Logger, NodeError> {
        let path_log = std::env::var(PATH_LOG).map_err(|_| {
            NodeError::EnvironVarNotFound("PATH_LOG not found in env vars".to_string())
        })?;
        Logger::new_from_path(&path_log)
    }

    /// Starts the logger thread.
    ///
    /// # Arguments
    ///
    /// * `receiver` - The receiver end of a channel used to receive log messages.
    /// * `path` - The path of the log file.
    /// * `sender` - The sender end of the channel used to receive log messages.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToOpenFile` if the log file could not be opened.
    fn start(receiver: Receiver<String>, path: &str) -> Result<(), NodeError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path)
            .map_err(|e| NodeError::FailedToOpenFile(format!("Failed to open log file: {}", e)))?;

        let mut buf_writer = BufWriter::new(file);

        thread::spawn(move || {
            for msg in receiver {
                match writeln!(buf_writer, "{}", msg) {
                    Ok(_) => {
                        if let Err(e) = buf_writer.flush() {
                            println!("Error flushing buffer: {}", e);
                        }
                    }
                    Err(e) => {
                        println!("Error writing to file: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Writes a log message to the log file.
    ///
    /// # Arguments
    ///
    /// * `msg` - The log message to write.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if the log message could not be sent to the logger thread.
    pub fn log(&self, msg: String) -> Result<(), NodeError> {
        self.sender
            .send(msg)
            .map_err(|_| NodeError::FailedToSendMessage("Failed to send".to_string()))
    }
}
