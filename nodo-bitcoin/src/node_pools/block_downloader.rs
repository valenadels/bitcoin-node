use std::{
    net::TcpStream,
    path::Path,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::{
    block::{block_hash::BlockHash, validate_and_save_block},
    connectors::peer_connector::receive_message,
    constants::MAX_FAILED_COUNT,
    constants::MSG_BLOCK,
    header::Header,
    logger::Logger,
    messages::{block_message::BlockMessage, get_data_message::GetDataMessage},
    node::{
        connect_to_ip,
        message_type::MessageType,
        receive_messages::{
            receive_addr_message, receive_feefilter_message, receive_inv_message,
            receive_not_found_message, send_pong_message,
        },
    },
    node_error::NodeError,
    ui::ui_message::UIMessage,
    utils::Utils,
};
use std::time::Duration;

/// A worker thread in the thread pool.
pub struct BlockDownloader {
    /// The `JoinHandle` of the worker thread.
    thread: thread::JoinHandle<TcpStream>,
}

impl BlockDownloader {
    /// Creates a new worker thread with the given ID.
    /// Inside the thread, an infinite loop is executed which waits to receive
    /// a task from the mpsc::Receiver associated with the receiver object.
    ///  When a task is received, it is executed by calling the job function.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the worker thread.
    /// * `receiver` - The `mpsc::Receiver` that the worker thread should receive jobs from.
    ///   The mpsc::Receiver used here is protected by a Mutex object, which ensures
    ///   that only one thread can access it at any given time.
    ///   An Arc object is also used to ensure that the reference to the mpsc::Receiver
    ///   is safely available across multiple threads.
    ///   This allows multiple workers to receive tasks concurrently without causing concurrency issues.
    /// * `ip` - The IP address of the node to connect to.
    /// * `sender` - The `mpsc::Sender` that the worker thread should send jobs to.
    /// * `logger` - The `Logger` instance to be used by the `BlockDownloader` instance to send received blocks.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToCreateThread` if there is an issue creating the worker thread.
    pub fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<BlockHash>>>,
        ip: std::net::SocketAddr,
        failed_sender: mpsc::Sender<BlockHash>,
        logger: Arc<Mutex<Logger>>,
        ui_sender: glib::Sender<UIMessage>,
    ) -> Result<BlockDownloader, NodeError> {
        let builder = thread::Builder::new();
        let mut failed_count: usize = 0;
        let logger_ = logger
            .lock()
            .map_err(|_| NodeError::FailedToLog("Failed to acquire lock on logger".to_string()))?
            .clone();
        let mut stream = Self::connect_to_node(ip, id, &logger_)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .map_err(|_| NodeError::ReadTimeoutFromStream("Failed to set timeout".to_string()))?;
        let thread = builder
            .spawn(move || {
                loop {
                    let result = Self::process_hash_and_download(
                        &receiver,
                        &mut stream,
                        id,
                        &failed_sender,
                        &mut failed_count,
                        &logger,
                        &ui_sender,
                    );
                    match result {
                        None => {
                            Self::delete_timeout(&mut stream).unwrap_or(()); //delete timeout so that it doesn't affect broadcasting
                            break;
                        }
                        Some(_) => {
                            if failed_count > MAX_FAILED_COUNT {
                                println!(
                                    "Killing thread {} too many failures: {}",
                                    id, failed_count
                                );
                                Self::delete_timeout(&mut stream).unwrap_or(());
                                break;
                            } else {
                                continue;
                            }
                        }
                    }
                }
                stream
            })
            .map_err(|_| NodeError::FailedToCreateThread("Failed to create thread".to_string()))?;

        Ok(BlockDownloader { thread })
    }

    /// Waits for the worker thread to finish execution.
    /// Returns a `Result` containing the `TcpStream` returned by the worker thread on success,
    /// or a `NodeError` on failure.
    /// # Errors
    /// Returns a `NodeError::FailedToJoinThread` variant if the thread join fails.
    pub fn join(self) -> Result<TcpStream, NodeError> {
        let result = self
            .thread
            .join()
            .map_err(|_| NodeError::FailedToJoinThread("Failed to join thread".to_string()))?;

        Ok(result)
    }

    /// Downloads a block from a peer given the block's hash.
    ///
    /// The function first sets a read stimeout on the stream and then sends a getdata message to request the
    /// block with the given hash from the peer. Then it enters into a loop to receive messages from the peer
    /// until it receives the requested block. The loop handles different types of messages that may be received
    /// from the peer and ignores all messages that are not relevant to the block download. Once the block is
    /// received, the function returns the block bytes in a Vec.
    ///
    /// # Arguments
    ///
    /// * stream - A mutable reference to a TcpStream representing the connection to the peer.
    /// * hash_bytes - The hash of the block to be downloaded from the peer.
    /// * logger - A reference to the logger instance to be used by the function handle_block_download to log blocks.
    ///
    /// # Errors
    ///
    /// Returns a NodeError if any error occurs while setting the read timeout on the stream, sending the
    /// getdata message, or receiving any of the messages from the peer during the block download.
    pub fn block_download(
        stream: &mut TcpStream,
        hash_bytes: BlockHash,
        logger: &Arc<Mutex<Logger>>,
    ) -> Result<Vec<u8>, NodeError> {
        let data_message = GetDataMessage::new(1, MSG_BLOCK, hash_bytes)?;
        data_message.send_message(stream)?;
        Self::handle_block_download(stream, logger)
    }

    /// Handles the block download process over a TCP stream and performs corresponding actions.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` to receive and send messages.
    /// * `logger` - A reference to the logger instance to be used to log blocks.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<u8>` representing the downloaded block on success,
    /// or a `NodeError` on failure.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if there was an error while receiving, handling, or sending messages.
    fn handle_block_download(
        stream: &mut TcpStream,
        logger: &Arc<Mutex<Logger>>,
    ) -> Result<Vec<u8>, NodeError> {
        loop {
            if !Utils::is_tcpstream_connected(stream) {
                return Err(NodeError::FailedToConnect(
                    "The TCP stream is not connected anymore".to_string(),
                ));
            }
            let mut header = Header::new(stream)?;
            let command_name = header.extract_command_name()?;

            logger
                .lock()
                .map_err(|e| NodeError::FailedToLog(format!("{}", e)))?
                .log(format!(
                    "Receiving command in block download: {:?}",
                    command_name
                ))?;
            println!("Receiving command in blocks download: {:?}", command_name);

            match command_name {
                MessageType::Headers => {
                    println!("Received headers message");
                    continue;
                }
                MessageType::Block => {
                    println!("Recieved a block message");
                    let block_bytes = receive_message(stream, header.payload_size())?;
                    return Ok(block_bytes);
                }
                MessageType::Ping => {
                    println!("Handle ping message");
                    send_pong_message(stream, &header)?;
                    continue;
                }
                MessageType::Pong => {
                    println!("Handle pong message");
                    continue;
                }
                MessageType::SendHeaders => {
                    println!("Handle sendheaders message");
                    continue;
                }
                MessageType::Addr => {
                    println!("Recieved an addr message");
                    receive_addr_message(stream, &header)?;
                    continue;
                }
                MessageType::FeeFilter => {
                    println!("Recieved a feefilter message");
                    receive_feefilter_message(stream, &header)?;
                    continue;
                }
                MessageType::Inv => {
                    println!("Recieved a inv message");
                    receive_inv_message(stream, &header)?;
                    continue;
                }
                MessageType::NotFound => {
                    println!("Sync node does not have the block");
                    return Err(receive_not_found_message(stream, &header));
                }
                _ => {
                    println!("Command not supported");
                    receive_message(stream, header.payload_size())?;
                }
            }
        }
    }

    /// Downloads a block from the provided TCP stream with the given block hash,
    /// saves it to the specified file path, and sends the block hash to the provided
    /// sender. If an error occurs during the download, the function retries by
    /// sending the block hash to the sender again.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` through which to download
    ///              the block.
    /// * `block_hash` - An array of 32 bytes representing the hash of the block to
    ///                  download.
    /// * `path` - A string specifying the file path to which to save the downloaded
    ///            block.
    /// * `sender` - An `Arc` wrapped `Mutex` containing an `mpsc::Sender` for sending
    ///              block hashes to a receiver.
    /// * `id` - A reference to an integer identifying the current downloader.
    /// * `failed_sender` - An optional `mpsc::Sender` for sending block hashes to a receiver
    ///                     in case of a failed download.
    /// * `failed_count` - A mutable reference to an integer representing the number of failed
    ///                   downloads.
    /// * `logger` - A reference to the logger instance to be used to log blocks.
    /// # Errors
    /// Returns a `NodeError` if there was an error while downloading the block or saving it to
    /// the specified file path.
    fn download_and_save(
        stream: &mut TcpStream,
        block_hash: BlockHash,
        path: String,
        id: &usize,
        failed_sender: Option<mpsc::Sender<BlockHash>>,
        failed_count: &mut usize,
        logger: &Arc<Mutex<Logger>>,
    ) -> Result<(), NodeError> {
        match Self::block_download(stream, block_hash, logger) {
            Ok(block) => {
                println!("Downloader {} downloaded block {:?}", id, path);
                logger
                    .lock()
                    .map_err(|e| NodeError::FailedToLog(format!("{}", e)))?
                    .log(format!("Downloaded block to {:?} from thread {}", path, id))?;
                if let Err(err) = Self::save_block(block, path) {
                    println!("Error save block: {:?}", err);
                };
                Ok(())
            }
            Err(err) => {
                println!("Error: {:?}. Queuing to failed channel..", err);
                *failed_count += 1;
                if let Some(sender) = failed_sender {
                    match sender.send(block_hash) {
                        Ok(_) => (),
                        Err(err) => println!("Error queuing to failed channel: {:?}", err),
                    }
                }
                Err(NodeError::FailedToDownloadBlock(
                    "Failed to download block".to_string(),
                ))
            }
        }
    }

    /// Downloads the block with the given hash from the peer listening on the provided
    /// TCP stream if it has not already been downloaded and saved to disk. If the block is
    /// successfully downloaded and saved, its hash is sent to the provided sender using
    /// an `mpsc::Sender`.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - A 32-byte array containing the hash of the block to download.
    /// * `stream` - A mutable reference to a `TcpStream` on which to download the block.
    /// * `failed_sender` - An `Arc` wrapped `Mutex` containing an `mpsc::Sender` for sending failed block hashes.
    /// * `id` - An identifier for the downloader.
    /// * `failed_count` - A mutable reference to an integer representing the number of failed downloads.
    /// * `logger` - A reference to the logger instance to be used to log blocks.
    /// # Errors
    /// Returns a `NodeError` if there was an error while downloading the block or saving it to
    /// the specified file path.
    pub fn download_block(
        block_hash: BlockHash,
        stream: &mut TcpStream,
        id: usize,
        failed_sender: Option<&mpsc::Sender<BlockHash>>,
        failed_count: &mut usize,
        logger: &Arc<Mutex<Logger>>,
        ui_sender: &glib::Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        let path = match BlockMessage::block_path(&block_hash) {
            Some(value) => value,
            None => {
                return Err(NodeError::FailedToDownloadBlock(
                    "Failed to get block path".to_string(),
                ))
            }
        };
        if !Path::new(&path).exists() {
            println!("Downloading block {:?} from downloader {}", path, id);
            Self::download_and_save(
                stream,
                block_hash,
                path,
                &id,
                failed_sender.cloned(),
                failed_count,
                logger,
            )?;
        } else {
            println!("Won't download block {:?}, already downloaded", path);
        }

        ui_sender
            .send(UIMessage::UpdateBlocksProgress)
            .unwrap_or_else(|e| {
                println!("Error sending update progress: {:?}", e);
            });
        Ok(())
    }

    /// Connects to a remote node at the provided IP address and performs a handshake to establish
    /// communication. Returns a `TcpStream` object that can be used to communicate with the remote
    /// node.
    ///
    /// # Arguments
    ///
    /// * `ip` - A `SocketAddr` representing the IP address of the remote node.
    /// * `id` - An identifier for the downloader.
    /// * `logger` - A reference to the logger instance to be used to log blocks.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToConnect` if there is an error connecting to or performing a handshake with the
    /// remote node.
    fn connect_to_node(
        ip: std::net::SocketAddr,
        id: usize,
        logger: &Logger,
    ) -> Result<TcpStream, NodeError> {
        let stream = connect_to_ip(&ip, logger).ok_or(NodeError::FailedToConnect(format!(
            "Failed to connect to peer {} in thread pool",
            ip
        )))?;
        println!("Downloader {} connected to peer {}", id, ip);
        Ok(stream)
    }

    /// If there is a hash in the channel, passes it to `download_block` to download the block.
    /// If the channel is closed, returns `None`, meaning that the IBD is finished.
    /// # Arguments
    /// * `receiver` - An `Arc` wrapped `Mutex` containing an `mpsc::Receiver` for receiving block hashes.
    /// * `stream` - A mutable reference to a `TcpStream` on which to download the block.
    /// * `id` - An identifier for the downloader.
    /// * `failed_sender` - An `Arc` wrapped `Mutex` containing an `mpsc::Sender` for sending failed block hashes.
    /// * `failed_count` - A mutable reference to an integer representing the number of failed downloads.
    /// * `logger` - A reference to the logger instance to be used to log blocks.
    /// # Errors
    /// Returns `None` if the channel is closed, meaning that the IBD is finished.
    fn process_hash_and_download(
        receiver: &Arc<Mutex<mpsc::Receiver<BlockHash>>>,
        stream: &mut TcpStream,
        id: usize,
        failed_sender: &mpsc::Sender<BlockHash>,
        failed_count: &mut usize,
        logger: &Arc<Mutex<Logger>>,
        ui_sender: &glib::Sender<UIMessage>,
    ) -> Option<()> {
        let received_hash = receiver.lock().ok()?.recv().ok();

        if let Some(received_hash) = received_hash {
            let _result = Self::download_block(
                received_hash,
                stream,
                id,
                Some(failed_sender),
                failed_count,
                logger,
                ui_sender,
            );
        } else {
            println!(
                "Downloader {} finished because channel is closed. Finished IBD",
                id
            );
            return None;
        }

        Some(())
    }

    /// Saves a block to the specified file path and updates the UtxoSet.
    ///
    /// # Arguments
    ///
    /// * `block_bytes` - A vector of bytes representing the block to be saved.
    /// * `path` - A string representing the file path to save the block to.
    /// * `utxo_set` - An `Arc` wrapped `Mutex` containing a `UtxoSet` instance.
    /// * `logger` - A reference to the logger instance to be used to log blocks.
    ///
    /// # Errors
    ///
    /// Returns a 'NodeError' if the file could not be opened or written to or if there was an error with the UtxoSet.
    pub fn save_block(block_bytes: Vec<u8>, path: String) -> Result<(), NodeError> {
        validate_and_save_block(block_bytes, &path)?;
        print!("Saving block to {}...", path);
        Ok(())
    }

    /// Deletes the timeout from the stream set when downloading blocks in order to prevent nodes from taking so
    /// long to send block message.
    /// # Arguments
    /// * `stream` - A mutable reference to a `TcpStream` on which to delete the timeout.
    /// # Errors
    /// Returns a `NodeError::ReadTimeoutFromStream` if there is an error deleting the timeout.
    fn delete_timeout(stream: &mut TcpStream) -> Result<(), NodeError> {
        stream.set_read_timeout(None).map_err(|_| {
            NodeError::ReadTimeoutFromStream("Failed to reset read timeout".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read};

    use crate::node_pools::block_downloader::BlockDownloader;

    #[test]
    fn test_save_block() {
        let mut block = fs::OpenOptions::new()
            .read(true)
            .open(
                "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin",
            )
            .unwrap();
        let mut block_bytes = Vec::new();
        block.read_to_end(&mut block_bytes).unwrap();

        let path = "test_save_block.bin".to_string();
        let result = BlockDownloader::save_block(block_bytes.clone(), path);
        assert!(result.is_ok());
        let path = "test_save_block.bin".to_string();
        let result = BlockDownloader::save_block(block_bytes, path);
        assert!(result.is_err());
        fs::remove_file("test_save_block.bin").unwrap();
    }
}
