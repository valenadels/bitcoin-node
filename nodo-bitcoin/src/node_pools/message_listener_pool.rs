use glib::Sender;

use super::listener::MessageListener;
use crate::channels::wallet_channel::WalletChannel;
use crate::logger::Logger;
use crate::node_error::NodeError;
use crate::transactions::utxo_set::UtxoSet;
use crate::ui::ui_message::UIMessage;
use crate::utils::Utils;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub struct MessageListenerPool {
    /// The collection of worker threads that will execute jobs.
    pub listeners: Vec<MessageListener>,
}

impl MessageListenerPool {
    /// Creates a new `ThreadPool` instance from a vector of TCP streams.
    ///
    /// # Arguments
    ///
    /// * `size` - The number of threads in the thread pool.
    /// * `streams` - A vector of `TcpStream` instances.
    /// * `utxo_set` - The `UtxoSet` as an Arc Mutex instance to be used by the `BlockBroadcasting` instances.
    /// * `logger` - The `Logger` instance to be used by the `BlockBroadcasting` instances.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the newly created `ThreadPool` instance on success,
    /// or a `NodeError` on failure.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToCreateThread` variant if the thread pool size is 0.
    /// Returns a `NodeError::FailedToConnect` variant if no thread could be created.
    pub fn new(
        size: usize,
        connections: &Vec<TcpStream>,
        utxo_set_arc: Arc<Mutex<UtxoSet>>,
        ui_sender: Sender<UIMessage>,
        wallet_channel: WalletChannel,
        logger: Logger,
    ) -> Result<MessageListenerPool, NodeError> {
        if size == 0 {
            return Err(NodeError::FailedToCreateThread(
                "The size of the thread pool must be greater than 0".to_string(),
            ));
        }
        let mut downloaders = Vec::with_capacity(size);
        let mut id = 0;
        let wallet_channel_arc = Arc::new(Mutex::new(wallet_channel));

        let logger = Arc::new(Mutex::new(logger));
        for stream in connections {
            if !Utils::is_tcpstream_connected(stream) {
                continue;
            }
            match MessageListener::new(
                id,
                stream.try_clone().map_err(|_| {
                    NodeError::FailedToCloneStream(
                        "Failed to clone the TCP stream in msg listener".to_string(),
                    )
                })?,
                Arc::clone(&utxo_set_arc),
                Arc::clone(&wallet_channel_arc),
                ui_sender.clone(),
                Arc::clone(&logger),
            ) {
                Ok(downloader) => {
                    downloaders.push(downloader);
                    id += 1;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        if downloaders.is_empty() {
            return Err(NodeError::FailedToConnect(
                "Failed to create any thread".to_string(),
            ));
        }
        Ok(MessageListenerPool {
            listeners: downloaders,
        })
    }

    /// Joins all the TCP streams of the listeners and returns a vector of connected streams.
    ///
    /// # Arguments
    ///
    /// * `self` - The listener
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the vector of connected TCP streams if successful,
    /// or a `NodeError` if any of the listeners encountered an error during joining.
    pub fn join(self) -> Result<Vec<TcpStream>, NodeError> {
        let mut connections = Vec::with_capacity(self.listeners.len());
        for downloader in self.listeners {
            let result = downloader.join()?;
            connections.push(result);
        }

        Ok(connections)
    }
}
