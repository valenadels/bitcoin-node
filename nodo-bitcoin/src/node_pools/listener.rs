use std::{
    net::TcpStream,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use glib::Sender;

use crate::{
    block::{block_hash::BlockHash, validate_and_save_block_listener},
    channels::wallet_channel::WalletChannel,
    connectors::peer_connector::receive_message,
    header::Header,
    logger::Logger,
    messages::block_message::BlockMessage,
    node::{
        message_type::MessageType,
        receive_messages::{
            receive_addr_message, receive_and_handle_inv_message, receive_feefilter_message,
            receive_tx_message, send_pong_message,
        },
        send_tx_to_wallet,
    },
    node_error::NodeError,
    transactions::{transaction::Transaction, utxo_set::UtxoSet},
    ui::ui_message::UIMessage,
    utils::Utils,
    wallet::node_wallet_message::NodeWalletMsg,
};

use super::{block_downloader::BlockDownloader, received_data_listener::ReceivedDataFromPeers};

/// A worker thread in the thread pool.
pub struct MessageListener {
    /// The `JoinHandle` of the worker thread.
    thread: thread::JoinHandle<TcpStream>,
}

impl MessageListener {
    /// Creates a new `MessageListener` instance from a TCP stream.
    ///
    /// # Arguments
    ///
    /// * `id` - An identifier for the `BlockBroadcaster`.
    /// * `stream` - A mutable reference to a `TcpStream`.
    /// * `utxo_set` - A shared reference to a `Mutex<UtxoSet>`.
    /// * `logger` - A shared reference to a `Mutex<Logger>`.
    /// * `wallet_channel` - A shared reference to a `Mutex<WalletChannel>`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the newly created `BlockBroadcaster` instance on success,
    /// or a `NodeError` on failure.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToCreateThread` variant if the thread creation fails.
    pub fn new(
        id: usize,
        mut stream: TcpStream,
        utxo_set: Arc<Mutex<UtxoSet>>,
        wallet_channel: Arc<Mutex<WalletChannel>>,
        ui_sender: Sender<UIMessage>,
        logger: Arc<Mutex<Logger>>,
    ) -> Result<MessageListener, NodeError> {
        let builder = thread::Builder::new();
        let thread = builder
            .spawn(move || {
                loop {
                    let result = Self::listen_to_new_messages(
                        &mut stream,
                        id,
                        &utxo_set,
                        &wallet_channel,
                        &ui_sender,
                        &logger,
                    );
                    match result {
                        None => break,
                        Some(_) => continue,
                    }
                }
                stream
            })
            .map_err(|_| NodeError::FailedToCreateThread("Failed to create thread".to_string()))?;

        Ok(MessageListener { thread })
    }

    /// Listens to new blocks or txns from a TCP stream.
    /// Handles the messages received from the stream depending on the message type.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream`.
    /// * 'logger' - A shared reference to a `Mutex<Logger>`.
    ///
    /// # Returns
    ///
    /// A vector of bytes containing the serialized block message.
    /// If a txn message is received, queues the txn message to the wallet channel.
    pub fn broadcasting_start(
        stream: &mut TcpStream,
        logger: &Arc<Mutex<Logger>>,
    ) -> Result<ReceivedDataFromPeers, NodeError> {
        stream.set_read_timeout(None).map_err(|_| {
            NodeError::ReadTimeoutFromStream("Trying to reset timeout failed".to_string())
        })?;

        loop {
            let mut header = Header::new(stream).map_err(|e| {
                NodeError::FailedToReadExact(format!("Stream: {:?}: {:?}", stream.peer_addr(), e))
            })?;
            let command_name = header.extract_command_name()?;
            logger
                .lock()
                .map_err(|e| NodeError::FailedToLog(format!("{}", e)))?
                .log(format!(
                    "Receiving command in blocks broadcasting: {:?}",
                    command_name
                ))?;
            println!(
                "Receiving command in blocks broadcasting: {:?}",
                command_name
            );

            match command_name {
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
                    match receive_and_handle_inv_message(stream, &header) {
                        Ok(block_hash) => {
                            if block_hash.is_empty() {
                                continue;
                            }
                            return Ok(ReceivedDataFromPeers::BlockHash(block_hash));
                        }
                        Err(e) => {
                            println!("Error in handling inv message: {:?}", e);
                            continue;
                        }
                    }
                }
                MessageType::Tx => {
                    println!("Recieved a tx message");
                    match receive_tx_message(stream) {
                        Ok(tx) => return Ok(ReceivedDataFromPeers::Transaction(tx)),
                        Err(e) => {
                            println!("Error in handling tx message: {:?}", e);
                            continue;
                        }
                    }
                }
                _ => {
                    println!("Command not supported");
                    receive_message(stream, header.payload_size())?;
                }
            }
        }
    }

    /// Listens to new block broadcasts on the provided TCP stream and downloads the block
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` on which to listen for block broadcasts.
    /// * `id` - A reference to an integer identifying the current downloader.
    /// * `handler` - A reference to a `Handler` instance for handling the block broadcasting.
    /// * `utxo_set` - An `Arc` wrapped `Mutex` containing a `UtxoSet` instance.
    /// * `logger` - A reference to an Arc Mutex `Logger` for logging.
    /// * `wallet_channel` - A reference to an Arc Mutex `WalletChannel` for sending transactions to the wallet.
    /// # Returns
    /// Returns a `Result` containing `()` on success, or a `NodeError` on failure.
    /// # Errors
    /// Returns a `NodeError::FailedToBroadcastBlock` variant if the block broadcasting fails.
    fn listen_to_new_messages(
        stream: &mut TcpStream,
        id: usize,
        utxo_set: &Arc<Mutex<UtxoSet>>,
        wallet_channel: &Arc<Mutex<WalletChannel>>,
        ui_sender: &Sender<UIMessage>,
        logger: &Arc<Mutex<Logger>>,
    ) -> Option<()> {
        match Self::broadcasting_start(stream, logger) {
            Ok(ReceivedDataFromPeers::BlockHash(new_block_hash)) => {
                Self::download_block(
                    new_block_hash,
                    stream,
                    id,
                    utxo_set,
                    logger,
                    wallet_channel,
                    ui_sender,
                );
                Some(())
            }
            Ok(ReceivedDataFromPeers::Transaction(tx)) => {
                match Self::process_transaction(tx, logger, wallet_channel) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error in processing transaction: {:?}", e);
                    }
                }
                Some(())
            }
            Err(e) => {
                println!("Error in handling new messages: {:?}", e);
                None
            }
        }
    }

    /// Saves a block to the specified file path. If the file already exists, this function will
    /// return an error.
    ///
    /// # Arguments
    ///
    /// * `block_bytes` - A vector of bytes representing the block to be saved.
    /// * `path` - A string representing the file path to save the block to.
    /// * `utxo_set` - An `Arc` wrapped `Mutex` containing a `UtxoSet` instance.
    ///
    /// # Errors
    ///
    /// Returns a 'NodeError' if the file could not be opened or written to.
    pub fn save_block(
        block_bytes: Vec<u8>,
        path: String,
        utxo_set: &Arc<Mutex<UtxoSet>>,
        wallet_channel: &Arc<Mutex<WalletChannel>>,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        validate_and_save_block_listener(block_bytes, &path, ui_sender)?;
        println!("Saving block to {}...", path);
        utxo_set
            .lock()
            .map_err(|_| NodeError::UtxoSetMutexError("UtxoSet mutex poisoned".to_string()))?
            .update(&path)?;

        wallet_channel
            .lock()
            .map_err(|_| NodeError::FailedToConvert("failed to lock wallet channel".to_string()))?
            .send(NodeWalletMsg::NewBlock(path))?;

        Ok(())
    }

    /// Downloads a block from the provided TCP stream and saves it to the specified file path.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` on which to listen for block broadcasts.
    /// * `block_hash` - A 32 byte array representing the hash of the block to be downloaded.
    /// * `id` - A reference to an integer identifying the current downloader.
    /// * `utxo_set` - An `Arc` wrapped `Mutex` containing a `UtxoSet` instance.
    /// * `logger` - A reference to an Arc Mutex `Logger` for logging.
    /// * `wallet_channel` - A reference to an Arc Mutex `WalletChannel` for sending transactions to the wallet.
    /// * `ui_sender` - A reference to a `Sender` for sending messages to the UI thread.
    pub fn download_block(
        block_hash: Vec<u8>,
        stream: &mut TcpStream,
        id: usize,
        utxo_set: &Arc<Mutex<UtxoSet>>,
        logger: &Arc<Mutex<Logger>>,
        wallet_channel: &Arc<Mutex<WalletChannel>>,
        ui_sender: &Sender<UIMessage>,
    ) {
        println!("New block hash: {:?}", block_hash);
        let new_block_hash = block_hash.try_into().unwrap_or([0; 32]);

        let path = match BlockMessage::block_path(&new_block_hash) {
            Some(value) => value,
            None => return,
        };
        if !Path::new(&path).exists() {
            println!("Downloading block {:?} from downloader {}", path, id);
            Self::download_and_save(
                stream,
                new_block_hash,
                path,
                &id,
                utxo_set,
                logger,
                (wallet_channel, ui_sender),
            );
        } else {
            println!("Won't download block {:?}, already downloaded", path);
        }
    }

    /// Downloads a block from the provided TCP stream and saves it to the specified file path.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` on which to listen for block broadcasts.
    /// * `block_hash` - A 32 byte array representing the hash of the block to be downloaded.
    /// * `path` - A string representing the file path to save the block to.
    /// * `id` - A reference to an integer identifying the current downloader.
    /// * `utxo_set` - An `Arc` wrapped `Mutex` containing a `UtxoSet` instance.
    /// * `logger` - A reference to an Arc Mutex `Logger` for logging.
    fn download_and_save(
        stream: &mut TcpStream,
        block_hash: BlockHash,
        path: String,
        id: &usize,
        utxo_set: &Arc<Mutex<UtxoSet>>,
        logger: &Arc<Mutex<Logger>>,
        channels: (&Arc<Mutex<WalletChannel>>, &Sender<UIMessage>),
    ) {
        match BlockDownloader::block_download(stream, block_hash, logger) {
            Ok(block) => {
                println!("Downloader {} downloaded block {:?}", id, path);
                if Self::save_block(block, path, utxo_set, channels.0, channels.1).is_err() {
                    println!("Didn't save block because other thread saved it");
                }
            }
            Err(err) => {
                println!("Error block_download: {:?}", err);
            }
        }
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

    /// Processes a new transaction.
    ///
    /// This function takes a `Transaction`, a reference to a `Logger` wrapped in an `Arc<Mutex>`,
    /// a reference to a `WalletChannel` wrapped in an `Arc<Mutex>`, and a reference to a `Sender<UIMessage>`.
    /// It prints the received transaction ID, sends a `NewTransaction` message to the UI thread,
    /// logs the transaction, and retrieves user addresses from the transaction.
    ///
    /// # Arguments
    ///
    /// * `tx` - A `Transaction` representing the new transaction to process.
    /// * `logger` - A reference to a `Logger` wrapped in an `Arc<Mutex>` for logging purposes.
    /// * `wallet_channel` - A reference to a `WalletChannel` wrapped in an `Arc<Mutex>` for communication with the wallet.
    /// * `ui_sender` - A reference to a `Sender<UIMessage>` for sending messages to the UI thread.
    fn process_transaction(
        tx: Transaction,
        logger: &Arc<Mutex<Logger>>,
        wallet_channel: &Arc<Mutex<WalletChannel>>,
    ) -> Result<(), NodeError> {
        let mut tx_id = tx.tx_id();
        tx_id.reverse();
        println!(
            "Received new transaction: {:?}",
            Utils::bytes_to_hex(&tx_id)
        );

        match logger
            .lock()
            .map_err(|_| NodeError::FailedToLog("Failed to lock logger in listener".to_string()))?
            .log(format!(
                "Received transaction: {:?}",
                Utils::bytes_to_hex(&tx_id)
            )) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to log new transaction: {:?}", e);
            }
        }

        send_tx_to_wallet(tx, wallet_channel)?;
        Ok(())
    }
}
