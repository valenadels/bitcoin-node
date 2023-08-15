use super::block_downloader::BlockDownloader;
use crate::block::block_hash::BlockHash;
use crate::logger::Logger;
use crate::node_error::NodeError;
use crate::ui::ui_message::UIMessage;
use std::net::{SocketAddr, TcpStream};
use std::sync::{mpsc, Arc, Mutex};

/// A thread pool that can execute jobs in parallel.
pub struct BlockDownloaderPool {
    /// The collection of worker threads that will execute jobs.
    pub block_downloaders: Vec<BlockDownloader>,
    /// The sender used to send hashes to the worker threads.
    pub hash_sender: Option<mpsc::Sender<BlockHash>>,
    /// The receiver used to receive failed hashes from the worker threads.
    pub failed_hash_receiver: Option<mpsc::Receiver<BlockHash>>,
    /// The sender used to send failed hashes to the main thread.
    pub failed_hash_sender: Option<mpsc::Sender<BlockHash>>,
}

/// Represents the result of a `ThreadPool::join` call.
/// It contains a vector of `TcpStream` representing the connections to the peers
/// and an `Option<mpsc::Receiver<[u8; 32]>>` used for receiving failed hashes in order to retry them in the main thread.
type JoinResult = (Vec<TcpStream>, Option<mpsc::Receiver<BlockHash>>);

impl BlockDownloaderPool {
    /// Creates a new thread pool with the given size.
    ///
    /// # Arguments
    ///
    /// * `size` - The desired number of worker threads in the pool. If `size` is less than or equal to 0, the default size will be used.
    /// * `ips` - A vector of `SocketAddr` representing the IP addresses to connect to.
    /// * `logger` - The `Logger` instance to be used by the `BlockDownloader` instances.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if there is an issue creating the thread pool. This can happen if the worker threads fail to initialize.
    pub fn new(
        size: usize,
        ips: &[SocketAddr],
        logger: Logger,
        ui_sender: &glib::Sender<UIMessage>,
    ) -> Result<BlockDownloaderPool, NodeError> {
        if size == 0 {
            return Err(NodeError::FailedToCreateThread(
                "The size of the thread pool must be greater than 0".to_string(),
            ));
        }
        let (sender, receiver) = mpsc::channel();
        let (failed_sender, failed_receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut downloaders = Vec::with_capacity(size);

        Self::create_downloaders(
            size,
            ips.to_vec(),
            receiver,
            failed_sender.clone(),
            &mut downloaders,
            logger,
            ui_sender,
        )?;

        if downloaders.is_empty() {
            return Err(NodeError::FailedToConnect(
                "Failed to connect to any peer".to_string(),
            ));
        }

        Ok(BlockDownloaderPool {
            block_downloaders: downloaders,
            hash_sender: Some(sender),
            failed_hash_receiver: Some(failed_receiver),
            failed_hash_sender: Some(failed_sender),
        })
    }

    /// Waits for all worker threads in the thread pool to finish and join their threads.
    ///
    /// This method blocks the calling thread until all worker threads have finished executing their
    /// jobs and joined their threads. If any worker thread encounters an error while executing a job,
    /// this method will return a `NodeError::FailedToJoinThread` containing the error message.
    ///
    /// # Errors
    ///
    /// This method returns a `NodeError::FailedToJoinThread` if any worker thread encounters an error while executing a job
    /// and joining its thread.
    pub fn join(mut self) -> Result<JoinResult, NodeError> {
        let mut connections = Vec::with_capacity(self.block_downloaders.len());
        for downloader in self.block_downloaders {
            let result = downloader.join()?;
            connections.push(result);
        }
        drop(self.hash_sender.take());
        drop(self.failed_hash_sender.take());
        Ok((connections, self.failed_hash_receiver))
    }

    pub fn close_channel(&mut self) {
        drop(self.hash_sender.take());
    }

    /// Handles the creation of `BlockDownloader` instances and populates the `downloaders` vector.
    ///
    /// # Arguments
    ///
    /// * `ips` - A vector of `SocketAddr` representing the IP addresses to connect to.
    /// * `id` - The initial identifier for the `BlockDownloader` instances.
    /// * `receiver` - An `Arc<Mutex<mpsc::Receiver<[u8; 32]>>>` used for receiving blocks.
    /// * `failed_sender_arc` - An `Arc<Mutex<mpsc::Sender<[u8; 32]>>>` used for sending failed blocks.
    /// * `failed_receiver` - An `Arc<Mutex<mpsc::Receiver<[u8; 32]>>>` used for receiving failed blocks.
    /// * `downloaders` - A mutable reference to a vector of `BlockDownloader` instances.
    /// * `logger` - The `Logger` instance to be used by the `BlockDownloader` instances.
    ///
    /// # Returns
    ///
    /// Returns `Some(Result<ThreadPool, NodeError>)` if there was an error during the creation of a `BlockDownloader`,
    /// otherwise returns `None`.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if the creation of a `BlockDownloader` fails due to an error other than a connection timeout.
    fn create_downloaders(
        size: usize,
        ips: Vec<SocketAddr>,
        receiver: Arc<Mutex<mpsc::Receiver<BlockHash>>>,
        failed_sender: mpsc::Sender<BlockHash>,
        downloaders: &mut Vec<BlockDownloader>,
        logger: Logger,
        ui_sender: &glib::Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        let mut id = 0;
        let logger_arc = Arc::new(Mutex::new(logger));

        for ip in ips {
            if ip.is_ipv6() {
                continue;
            }
            if id == size {
                break;
            }
            match BlockDownloader::new(
                id,
                Arc::clone(&receiver),
                ip,
                failed_sender.clone(),
                Arc::clone(&logger_arc),
                ui_sender.clone(),
            ) {
                Ok(downloader) => {
                    downloaders.push(downloader);
                    id += 1;
                }
                Err(err) => {
                    if let NodeError::FailedToConnect(_) = err {
                        println!(
                            "Failed to connect to peer at {}. Timeout. Retrying with other ip...",
                            ip
                        );
                        continue;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
        Ok(())
    }
}
