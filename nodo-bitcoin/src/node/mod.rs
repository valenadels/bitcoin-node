pub mod block_header_downloader;
pub mod message_type;
pub mod read;
pub mod receive_messages;
pub mod server;

use crate::{
    block::block_hash::BlockHash,
    block_header::BlockHeader,
    channels::wallet_channel::WalletChannel,
    config::load_app_config,
    connectors::peer_connector::{receive_message, send_message},
    constants::{BLOCKS_TO_SHOW, CONNECTION_TIMEOUT, LENGTH_HEADER_MESSAGE, MAX_RETRY_ATTEMPTS},
    header::Header,
    logger::Logger,
    messages::{
        tx_message::TxMessage,
        verack_message::{is_verack_message, VERACK_MESSAGE},
        version_message::VersionMessage,
    },
    node::read::obtain_ips,
    node_error::NodeError,
    node_pools::{
        block_downloader::BlockDownloader, block_downloader_pool::BlockDownloaderPool,
        message_listener_pool::MessageListenerPool,
    },
    transactions::{transaction::Transaction, utxo_set::UtxoSet},
    ui::ui_message::UIMessage,
    utils::Utils,
    wallet::{node_wallet_message::NodeWalletMsg, wallet_impl::Wallet},
};
use bitcoin_hashes::{sha256d, Hash};
use glib::Sender;

use std::{
    net::{SocketAddr, TcpStream},
    sync::{
        mpsc::{self},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use self::{
    block_header_downloader::BlockHeaderDownloader, read::read_initial_block_headers_from_file,
    server::start_server,
};

/// Initiates a handshake with a peer node.
///
/// This function performs the handshake process with a peer node over a TCP connection.
/// The handshake is a series of message exchanges that establish a connection and protocol version
/// compatibility between the nodes.
///
/// # Arguments
///
/// * `ip` - A reference to the socket address of the peer node.
/// * `stream` - A mutable reference to the TCP stream for communication with the peer node.
/// * `logger` - A reference to the logger used to log events during the handshake process.
///
/// # Errors
///
/// Returns an `Err(NodeError)` if any error occurs during the handshake process.
pub fn handshake(
    ip: &SocketAddr,
    stream: &mut TcpStream,
    logger: &Logger,
) -> Result<bool, NodeError> {
    let version_message = VersionMessage::create_version_message(ip)?;
    version_message.send_message(stream)?;

    let header = Header::new(stream)?;
    let payload_size = header.payload_size();
    receive_message(stream, payload_size)?;
    logger.log("Received version message".to_string())?;

    let transmiting_ver_ack = VERACK_MESSAGE.to_vec();

    send_message(stream, transmiting_ver_ack)?;
    let verack_received = &receive_message(stream, LENGTH_HEADER_MESSAGE)?;
    logger.log("Received verack message".to_string())?;

    Ok(is_verack_message(verack_received))
}

/// Initializes a connection with a node from the list of IP addresses.
///
/// # Arguments
///
/// * `ips` - A mutable reference to a vector of `SocketAddr` representing the list of IP addresses.
/// * `logger` - A logger to log the messages received.
/// # Returns
///
/// Returns a `Result` containing the established `TcpStream` if successful, or an `Err` variant
/// with a `NodeError` if an error occurs during the connection initialization.
pub fn init_connection(ips: &Vec<SocketAddr>, logger: &Logger) -> Result<TcpStream, NodeError> {
    for ip in ips {
        if ip.is_ipv6() {
            continue;
        }
        match connect_to_ip(ip, logger) {
            Some(stream) => {
                return Ok(stream);
            }
            None => {
                continue;
            }
        }
    }
    println!("No ips found");
    Err(NodeError::NoIpsFound("No ips found".to_string()))
}

/// Establishes a TCP connection to the specified IP address and performs a handshake.
///
/// The function tries to connect to the provided IP address with a timeout. If the connection is successful,
/// it performs a handshake with the remote node and returns a `TcpStream` if the handshake is successful. If an error
/// occurs during the connection or the handshake, the function will retry the connection up to a maximum number of attempts.
/// If the maximum number of attempts is reached, the function will return `None`.
///
/// # Arguments
///
/// * `ip` - A reference to a `SocketAddr` representing the IP address and port of the remote node.
/// * `logger` - A reference to a `Logger` instance used for logging purposes.
///
/// # Returns
///
/// Returns an `Option<TcpStream>` containing the established `TcpStream` if the connection and handshake are successful.
/// Otherwise, it returns `None` if there was an error during connection or if the handshake fails.
pub fn connect_to_ip(ip: &SocketAddr, logger: &Logger) -> Option<TcpStream> {
    for attempt in 1..=MAX_RETRY_ATTEMPTS {
        let stream = TcpStream::connect_timeout(ip, Duration::from_secs(CONNECTION_TIMEOUT));
        match stream {
            Ok(mut stream) => {
                if handshake_if_ok(ip, &mut stream, logger, attempt) {
                    return Some(stream);
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    println!("Timed out connecting to node with ip: {}", ip);
                }
                println!("Failed to connect: {} to node with ip: {}", e, ip);
            }
        }
    }
    println!(
        "Max retry attempts reached. Unable to connect to node with ip: {}",
        ip
    );
    None
}

/// Performs a handshake with the remote node.
/// The function verifies if the provided TCP stream is successfully connected to the specified IP address.
/// If the connection is successful, it performs a handshake with the remote node and returns true if the handshake is successful.
///
/// # Arguments
///
/// * `ip` - A reference to a `SocketAddr` representing the IP address and port of the remote node.
/// * `stream` - A reference to a `TcpStream` representing the established TCP connection.
/// * `logger` - A reference to a `Logger` instance used for logging purposes.
/// * `attempt` - The number of the current connection attempt.
///
/// # Returns
///
/// Returns `true` if the handshake is successful, otherwise returns `false`.
fn handshake_if_ok(ip: &SocketAddr, stream: &mut TcpStream, logger: &Logger, attempt: u64) -> bool {
    println!("Found a node with ip: {}", ip);
    if is_connection_ok(stream, ip) {
        if let Ok(handshake_result) = handshake(ip, stream, logger) {
            println!("Waiting for handshake...");
            if handshake_result {
                println!("Handshake successful");
                return true;
            } else {
                println!("Handshake failed");
                return false;
            }
        } else {
            println!("Handshake failed");
            return false;
        }
    } else {
        println!(
            "Connection attempt {}/{} failed to node with ip: {}",
            attempt, MAX_RETRY_ATTEMPTS, ip
        );
    }
    false
}

/// Checks if the TCP stream is connected to the specified IP address.
///
/// The function verifies if the provided TCP stream is successfully connected to the specified IP address.
///
/// # Arguments
///
/// * `stream` - A reference to a `TcpStream` representing the established TCP connection.
/// * `ip` - A reference to a `SocketAddr` representing the IP address and port of the remote node.
///
/// # Returns
///
/// Returns `true` if the TCP stream is connected to the specified IP address, otherwise returns `false`.
fn is_connection_ok(stream: &TcpStream, ip: &SocketAddr) -> bool {
    if Utils::is_tcpstream_connected(stream) {
        println!("Connected to node with ip: {}", ip);
        true
    } else {
        println!("Failed to connect to node with ip: {}", ip);
        false
    }
}

/// Downloads the initial block headers from a remote peer and returns them as a vector.
///
/// # Arguments
///
/// * `stream` - A `TcpStream` representing the network connection to the peer.
/// * `sender` - A reference to an `mpsc::Sender<[u8; 32]>` for sending the hashes to the queue.
/// * `logger` - A logger to log the messages received.
/// * `ui_sender` - A `glib::Sender<UIMessage>` for sending messages to the UI.
///
/// # Returns
///
/// Returns a `Result` containing a vector of `BlockHeader` if the download is successful, or an
/// `Err` variant with a `NodeError` if an error occurs during the download process.
pub fn initial_block_headers_download(
    stream: &mut TcpStream,
    sender: &mpsc::Sender<BlockHash>,
    ui_sender: &glib::Sender<UIMessage>,
    logger: &Logger,
) -> Result<Vec<BlockHeader>, NodeError> {
    let mut block_header_downloader = match BlockHeaderDownloader::new(stream) {
        Ok(bhd) => bhd,
        Err(e) => {
            println!("Failed to create block header downloader: {:?}", e);
            return Err(NodeError::TcpStreamNotConnected(
                "Failed to create block header downloader".to_string(),
            ));
        }
    };

    block_header_downloader.start(logger, ui_sender)?;
    let header_blocks = read_initial_block_headers_from_file()?;

    queue_hashes(&header_blocks, sender)?;

    Ok(header_blocks)
}

/// Queues the hashes of block headers for sending through a channel.
///
/// # Arguments
///
/// * `header_blocks` - A vector of `BlockHeader` objects representing the block headers.
/// * `sender` - A reference to an `mpsc::Sender<[u8; 32]>` for sending the hashes.
///
/// # Returns
///
/// Returns `Ok(())` if the hashes are successfully queued,
/// or a `NodeError` on failure.
///
/// # Errors
///
/// Returns a `NodeError` if there was an error while sending the hash through the channel.
fn queue_hashes(
    header_blocks: &[BlockHeader],
    sender: &mpsc::Sender<BlockHash>,
) -> Result<(), NodeError> {
    let mut i = 0;
    for block in header_blocks.iter() {
        let hash = sha256d::Hash::hash(&block.to_bytes()).to_byte_array();
        i += 1;
        sender.send(hash).map_err(|_| {
            NodeError::FailedToSendHash("Failed to send hash through channel".to_string())
        })?;
    }
    println!("Queued {} hashes", i);
    Ok(())
}

/// Downloads the blocks that could not be downloaded during the initial block download.
/// Will try to download the blocks from other connections if it is not in one peer.
///
/// # Arguments
///
/// * `failed_receiver` - A reference to an `mpsc::Receiver<[u8; 32]>` for receiving the hashes of the failed blocks.
/// * `connections` - A mutable reference to a vector of `TcpStream` representing the network connections to the peers.
/// * `logger` - A reference to a `Logger` for logging messages.
///
/// # Returns
///
/// Returns `Ok(())` if the blocks are successfully downloaded,
/// or a `NodeError` on failure.
pub fn retry_failed_blocks(
    failed_receiver: &mpsc::Receiver<BlockHash>,
    connections: &mut [TcpStream],
    logger: &Logger,
    ui_sender: &glib::Sender<UIMessage>,
) -> Result<(), NodeError> {
    loop {
        match failed_receiver.recv() {
            Ok(hash) => {
                println!("Retrying failed block with hash: {:?}", hash);
                if !retry_download(
                    connections,
                    hash,
                    Arc::new(Mutex::new(logger.clone())),
                    ui_sender,
                ) {
                    return Err(NodeError::FailedToDownloadBlock(
                        "Failed to download block from all connections".to_string(),
                    ));
                }
            }
            Err(_) => {
                println!("Finished retrying failed blocks");
                break;
            }
        }
    }

    Ok(())
}

/// Downloads the blocks that could not be downloaded during the initial block download.
/// Will try to download the blocks from other connections if it is not in one peer.
///
/// # Arguments
///
/// * `connections` - A mutable reference to a vector of `TcpStream` representing the network connections to the peers.
/// * `hash` - The hash of the block to download.
/// * `logger` - A reference to a `Logger` for logging messages.
///
/// # Returns
///
/// Returns `true` if the block is successfully downloaded,
/// or `false` on failure.
fn retry_download(
    connections: &mut [TcpStream],
    hash: BlockHash,
    logger: Arc<Mutex<Logger>>,
    ui_sender: &glib::Sender<UIMessage>,
) -> bool {
    let mut success = false;
    for conn in connections.iter_mut() {
        if let Err(err) =
            BlockDownloader::download_block(hash, conn, 0, None, &mut 0, &logger, ui_sender)
        {
            println!("Failed to download block from connection: {:?}", err);
            continue;
        }

        success = true;
        break;
    }
    success
}

/// Starts the server for the node.
pub fn run_server() -> JoinHandle<()> {
    thread::spawn(move || match start_server() {
        Ok(_) => println!("Server finished"),
        Err(_) => println!("Server failed"),
    })
}

/// Runs the Bitcoin node
///
/// # Arguments
///
/// * `ui_sender` - A `glib::Sender<UIMessage>` for sending messages to the UI.
/// * `wallet_ui_receiver` - A `mpsc::Receiver<UIMessage>` for receiving messages from the UI.
///
/// # Returns
///
/// If everything is ok, it will never return.
///
/// # Errors
///
/// Returns a NodeError if an error occurs while running the node.
pub fn run_node(
    ui_sender: glib::Sender<UIMessage>,
    wallet_ui_receiver: mpsc::Receiver<UIMessage>,
) -> Result<(), NodeError> {
    let (logger, stream, block_downloader_pool, ips) = initialize_node(&ui_sender)?;

    let (initial_block_headers, connections, _stream) =
        download_headers_and_blocks(block_downloader_pool, stream, ips, &ui_sender, &logger)?;

    let thread_server = run_server();

    broadcast(
        initial_block_headers,
        connections,
        ui_sender,
        wallet_ui_receiver,
        logger,
    )?;

    thread_server
        .join()
        .map_err(|_| NodeError::FailedToJoinThread("Failed to join server thread".to_string()))?;
    Ok(())
}

/// Starts the block and transaction broadcasting.
///
/// #Arguments
///
/// * `initial_block_headers` - A vector of `BlockHeader` objects representing the block headers.
/// * `connections` - A vector of `TcpStream` representing the network connections to the peers.
/// * `ui_sender` - A `glib::Sender<UIMessage>` for sending messages to the UI.
/// * `wallet_ui_receiver` - A `mpsc::Receiver<UIMessage>` for receiving messages from the UI.
/// * `logger` - A `Logger` for logging messages.
///
/// # Returns
///
/// Returns `Ok(())` if the blocks are successfully broadcasted, however it will never return unless an error occurs.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while broadcasting the blocks.
fn broadcast(
    initial_block_headers: Vec<BlockHeader>,
    connections: Vec<TcpStream>,
    ui_sender: Sender<UIMessage>,
    wallet_ui_receiver: mpsc::Receiver<UIMessage>,
    logger: Logger,
) -> Result<(), NodeError> {
    let utxo_set = UtxoSet::new_from_block_headers(initial_block_headers)?;
    let (wallet_channel, node_wallet_channel) = WalletChannel::create_pairs();
    let mut connection_to_peer = find_one_active_peer(&connections)?;

    let utxo_set_arc = Arc::new(Mutex::new(utxo_set));

    let broadcasting_pool = MessageListenerPool::new(
        connections.len(),
        &connections,
        Arc::clone(&utxo_set_arc),
        ui_sender.clone(),
        node_wallet_channel,
        logger,
    )?;

    let thread_wallet = thread::spawn(move || {
        match Wallet::run_wallet(
            Arc::clone(&utxo_set_arc),
            wallet_channel,
            wallet_ui_receiver,
            ui_sender,
            &mut connection_to_peer,
        ) {
            Ok(_) => println!("Wallet finished"),
            Err(_) => println!("Wallet failed"),
        }
    });

    broadcasting_pool.join()?;

    thread_wallet
        .join()
        .map_err(|_| NodeError::FailedToJoinThread("Failed to join wallet thread".to_string()))?;
    Ok(())
}

/// Finds one active peer to send to the wallet.
///
/// # Arguments
///
/// * `connections` - A vector of `TcpStream` representing the network connections to the peers.
///
/// # Returns
///
/// Returns a `TcpStream` representing the connection to the peer.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while finding a peer.
fn find_one_active_peer(connections: &[TcpStream]) -> Result<TcpStream, NodeError> {
    let connection_to_peer = connections
        .iter()
        .find(|conn| Utils::is_tcpstream_connected(conn))
        .ok_or(NodeError::FailedToConnect(
            "Failed to get a peer to send to wallet".to_string(),
        ))?
        .try_clone()
        .map_err(|_| NodeError::FailedToConnect("Failed to clone peer".to_string()))?;
    Ok(connection_to_peer)
}

/// Downloads all the block headers and blocks from the network from the config timestamp to now.
/// If blocks or headers are already downloaded, it will not download them again.
///
/// # Arguments
///
/// * `block_downloaders_pool` - A `BlockDownloaderPool` for downloading the blocks.
/// * `stream` - A `TcpStream` representing the network connection to the peer.
/// * `logger` - A reference to a `Logger` for logging messages.
/// * `ui_sender` - A `glib::Sender<UIMessage>` for sending messages to the UI.
///
/// # Returns
///
/// Returns a tuple containing the initial block headers, the connections and the stream.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while downloading the blocks.
fn download_headers_and_blocks(
    mut block_downloader_pool: BlockDownloaderPool,
    stream: TcpStream,
    ips: Vec<SocketAddr>,
    ui_sender: &glib::Sender<UIMessage>,
    logger: &Logger,
) -> Result<(Vec<BlockHeader>, Vec<TcpStream>, TcpStream), NodeError> {
    println!("Downloading headers and blocks");
    let sender = match block_downloader_pool.hash_sender.take() {
        Some(s) => s,
        None => Err(NodeError::FailedToSendMessage(
            "No sender found".to_string(),
        ))?,
    };

    let (initial_block_headers, stream) =
        ibh_download_or_retry_connection(ips, stream, sender, ui_sender, logger)?;

    send_block_headers_to_ui(ui_sender, &initial_block_headers)?;

    block_downloader_pool.close_channel();
    let (mut connections, failed_receiver) = block_downloader_pool.join()?;

    match failed_receiver {
        Some(receiver) => retry_failed_blocks(&receiver, &mut connections, logger, ui_sender)?,
        None => println!("No failed blocks found"),
    };
    Ok((initial_block_headers, connections, stream))
}

/// Attempts to download initial block headers (IBH) from multiple IP addresses or retries connections if needed.
///
/// The function tries to download initial block headers from a list of IP addresses. If the download is successful,
/// it returns the vector of block headers in a `Result::Ok`. If an error occurs during the download, the function
/// will retry the connection using the next available IP address until either the download succeeds or there are no more IPs to try.
///
/// # Arguments
///
/// * `ips` - A mutable vector of `SocketAddr` representing the list of IP addresses to attempt the download.
/// * `stream` - A mutable `TcpStream` representing the established TCP connection.
/// * `sender` - A `mpsc::Sender` for sending data to another thread (not directly used in this function).
/// * `logger` - A reference to a `Logger` instance used for logging purposes.
/// * `ui_sender` - A `glib::Sender<UIMessage>` for sending messages to the UI.
///
/// # Returns
///
/// Returns a `Result` containing either a tuple that contains a vector of `BlockHeader` and the stream, if the download is successful or a `NodeError` if the download fails.
///
/// # Errors
///
/// The function can return a `NodeError::FailedToConnect` if it exhausts all available IP addresses and cannot establish a successful connection.
fn ibh_download_or_retry_connection(
    mut ips: Vec<SocketAddr>,
    mut stream: TcpStream,
    sender: mpsc::Sender<[u8; 32]>,
    ui_sender: &glib::Sender<UIMessage>,
    logger: &Logger,
) -> Result<(Vec<BlockHeader>, TcpStream), NodeError> {
    // In order to retry the current connection in case of error
    add_curr_ip_to_ips(&stream, &mut ips)?;

    while !ips.is_empty() {
        match initial_block_headers_download(&mut stream, &sender, ui_sender, logger) {
            Ok(ibh) => {
                return Ok((ibh, stream));
            }
            Err(e) => {
                if let Some(next_ip) = next_ipv4(&mut ips) {
                    println!(
                        "Retrying IBH download with ip: {} because of error {:?}",
                        next_ip, e
                    );
                    stream = match connect_to_ip(&next_ip, logger) {
                        Some(s) => s,
                        None => {
                            continue;
                        }
                    }
                } else {
                    println!("No more ips to retry IBH download. Please re-run the node.");
                    break;
                }
            }
        }
    }

    println!("No more ips to retry IBH download. Please re-run the node.");
    Err(NodeError::FailedToConnect(
        "No more ips to retry IBH download.".to_string(),
    ))
}

/// Adds the current ip to the list of ips.
///
/// # Arguments
///
/// * `stream` - A mutable `TcpStream` representing the established TCP connection.
/// * `ips` - A mutable reference to a vector of `SocketAddr` representing the list of IP addresses.
///
/// # Returns
///
/// Returns `Ok(())` if the current ip is successfully added to the list of ips.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while adding the current ip to the list of ips.
fn add_curr_ip_to_ips(stream: &TcpStream, ips: &mut Vec<SocketAddr>) -> Result<(), NodeError> {
    let current_ip = stream
        .peer_addr()
        .unwrap_or(ips.pop().ok_or(NodeError::FailedToConnect(
            "No more ips to retry IBH download.".to_string(),
        ))?);
    ips.push(current_ip);
    Ok(())
}

/// Gets the next ipv4 address from the list of ips.
///
/// # Arguments
///
/// * `ips` - A mutable reference to a vector of `SocketAddr` representing the list of IP addresses.
///
/// # Returns
///
/// Returns a `SocketAddr` representing the next ipv4 address.
/// (Changes the list of ips, because it pops the last element from the list)
fn next_ipv4(ips: &mut Vec<SocketAddr>) -> Option<SocketAddr> {
    loop {
        match ips.pop() {
            Some(ip) => {
                if ip.is_ipv4() {
                    return Some(ip);
                }
            }
            None => return None,
        }
    }
}

/// Gets the ips from de DNS and loads the app config.
/// Creates a thread pool for downloading the blocks.
/// Returns a tuple containing the logger, the network connection, the thread pool and the IPS.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while initializing the node.
fn initialize_node(
    ui_sender: &glib::Sender<UIMessage>,
) -> Result<(Logger, TcpStream, BlockDownloaderPool, Vec<SocketAddr>), NodeError> {
    load_app_config(Some(ui_sender))?;
    println!("Loaded app config");
    let ips = obtain_ips()?;
    let logger = Logger::new()?;
    let stream = init_connection(&ips, &logger)?;
    let pool = BlockDownloaderPool::new(ips.len(), &ips, logger.clone(), ui_sender)?;
    println!("Created thread pool");
    Ok((logger, stream, pool, ips))
}

/// Sends block headers from the config timestamp to the UI.
///
/// # Arguments
///
/// * `ui_sender` - A reference to a `glib::Sender<UIMessage>` for sending messages to the UI.
/// * `initial_block_headers` - A slice of `BlockHeader` objects representing the block headers.
///
/// # Returns
///
/// Returns `Ok(())` if the block headers are successfully sent to the UI.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while sending the block headers to the UI.
fn send_block_headers_to_ui(
    ui_sender: &glib::Sender<UIMessage>,
    initial_block_headers: &[BlockHeader],
) -> Result<(), NodeError> {
    ui_sender
        .send(UIMessage::TotalBlocksToDownload(
            initial_block_headers.len().try_into().map_err(|_| {
                NodeError::FailedToSendMessage(
                    "Failed to send total blocks to download because usize conversion failed"
                        .to_string(),
                )
            })?,
        ))
        .map_err(|_| {
            NodeError::FailedToSendMessage("Failed to send total blocks to download".to_string())
        })?;

    let last_10k_blocks =
        initial_block_headers[initial_block_headers.len() - BLOCKS_TO_SHOW..].to_owned();
    ui_sender
        .send(UIMessage::InitialBlockHeaders(last_10k_blocks))
        .map_err(|_| {
            NodeError::FailedToSendMessage("Failed to send initial block headers".to_string())
        })?;
    Ok(())
}

/// Broadcasts a created transaction to the network.
///
/// # Arguments
///
/// * `transaction` - A `Transaction` object representing the transaction to be broadcasted.
/// * `connection` - A `TcpStream` representing the network connection to the peer.
///
/// # Returns
///
/// Returns `Ok(())` if the transaction is successfully broadcasted.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while broadcasting the transaction.
pub fn broadcast_transaction(
    transaction: Transaction,
    connection: &mut TcpStream,
) -> Result<(), NodeError> {
    TxMessage::send_tx_message(&transaction, connection)?;
    Ok(())
}

/// Extracts user addresses from a transaction and performs address validation.
///
/// This function takes a `Transaction` and a reference to a `WalletChannel` wrapped in an `Arc<Mutex>`.
/// It sends a user addresses message to the wallet channel, retrieves the user addresses
/// from the wallet channel using the `handle_wallet_addresses` function,
/// and then checks if the transaction contains any of the user addresses.
///
/// # Arguments
///
/// * `tx` - A `Transaction` representing the transaction to extract addresses from.
/// * `wallet_channel` - A reference to a `WalletChannel` wrapped in an `Arc<Mutex>` for communication with the wallet.
///
/// # Errors
///
/// Returns an `Err` variant of `NodeError` if there are any errors encountered during the process.
pub fn send_tx_to_wallet(
    tx: Transaction,
    wallet_channel: &Arc<Mutex<WalletChannel>>,
) -> Result<(), NodeError> {
    wallet_channel
        .lock()
        .map_err(|_| {
            NodeError::FailedToReceiveMessage("Failed to lock wallet channel".to_string())
        })?
        .send(NodeWalletMsg::NewTransaction(tx))
        .map_err(|e| {
            NodeError::FailedToSendMessage(format!("Failed to send tx to wallet {:?}", e))
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        fs::File,
        io::{BufRead, BufReader},
        net::{IpAddr, Ipv4Addr, SocketAddr},
    };

    use bitcoin_hashes::{sha256d, Hash};

    use crate::{
        block_header::BlockHeader,
        config::{load_app_config, parse_line},
        constants::{COMMAND_NAME_VERSION, DEFAULT_CONFIG, TESTNET_MAGIC_BYTES},
        header::Header,
        messages::version_message::VersionMessage,
        node::read::obtain_ips,
        node_error::NodeError,
        transactions::utxo_set::UtxoSet,
    };

    fn load_default_config() -> Result<(), NodeError> {
        let file = File::open(DEFAULT_CONFIG)
            .map_err(|_| NodeError::FailedToOpenFile("Failed to open config file".to_string()))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line_content =
                line.map_err(|_| NodeError::FailedToRead("Failed to read line".to_string()))?;
            let (key, value) = parse_line(&line_content)?;
            env::set_var(key, value);
        }
        Ok(())
    }

    #[test]
    fn test_create_header() -> Result<(), NodeError> {
        load_default_config()?;
        let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8223);
        let version = VersionMessage::new(&ip)?;
        let version_message_bytes = VersionMessage::to_bytes(&version);
        let header = Header::create_header(&version_message_bytes, COMMAND_NAME_VERSION).unwrap();
        assert_eq!(header.len(), 24);
        assert_eq!(header[0..4].to_vec(), TESTNET_MAGIC_BYTES.to_vec());
        assert_eq!(
            header[4..11].to_vec(),
            COMMAND_NAME_VERSION.as_bytes().to_vec(),
            "Command name does not match"
        );
        assert_eq!(
            header[16..20].to_vec(),
            (version_message_bytes.len() as u32).to_le_bytes().to_vec(),
            "Payload size does not match"
        );
        let checksum_vec = sha256d::Hash::hash(&version_message_bytes).to_byte_array();
        match checksum_vec.get(0..4) {
            Some(c) => assert_eq!(
                header[20..24].to_vec(),
                c.to_vec(),
                "Checksum does not match"
            ),
            None => {
                return Err(NodeError::FailedToCreateHeaderField(
                    "Error al calcular el checksum".to_string(),
                ))
            }
        };
        Ok(())
    }
    #[test]
    fn test_get_ips() -> Result<(), NodeError> {
        load_app_config(None)?;
        let ips = obtain_ips()?;
        assert!(ips.len() > 0);

        Ok(())
    }

    #[test]
    fn test_utxo_set() -> Result<(), NodeError> {
        load_default_config()?;
        let block_header1 = BlockHeader {
            version: 541065216,
            prev_blockhash: [
                140, 91, 243, 132, 49, 171, 165, 229, 63, 197, 82, 189, 43, 175, 58, 194, 125, 114,
                109, 191, 213, 62, 105, 172, 18, 0, 0, 0, 0, 0, 0, 0,
            ],
            merkle_root_hash: [
                255, 9, 28, 10, 250, 45, 93, 244, 175, 95, 97, 96, 184, 48, 200, 92, 89, 219, 30,
                30, 231, 177, 182, 7, 54, 122, 85, 115, 101, 169, 35, 220,
            ],
            timestamp: 1684364618,
            n_bits: 422038156,
            nonce: 2517263891,
            hash: [
                71, 105, 174, 91, 7, 137, 212, 160, 194, 69, 156, 86, 109, 126, 215, 239, 8, 28,
                120, 180, 117, 42, 134, 239, 14, 0, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
        };

        let block_header2 = BlockHeader {
            version: 543162368,
            prev_blockhash: [
                71, 105, 174, 91, 7, 137, 212, 160, 194, 69, 156, 86, 109, 126, 215, 239, 8, 28,
                120, 180, 117, 42, 134, 239, 14, 0, 0, 0, 0, 0, 0, 0,
            ],
            merkle_root_hash: [
                157, 211, 187, 115, 192, 213, 182, 85, 108, 129, 43, 172, 208, 132, 99, 241, 221,
                54, 32, 218, 97, 114, 32, 43, 232, 113, 89, 202, 99, 123, 205, 94,
            ],
            timestamp: 1684364757,
            n_bits: 422038156,
            nonce: 1137852494,
            hash: [
                64, 1, 236, 182, 35, 169, 38, 236, 81, 114, 124, 60, 29, 225, 243, 254, 226, 111,
                164, 71, 56, 243, 50, 92, 39, 0, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
        };

        let block_header3 = BlockHeader {
            version: 541065216,
            prev_blockhash: [
                64, 1, 236, 182, 35, 169, 38, 236, 81, 114, 124, 60, 29, 225, 243, 254, 226, 111,
                164, 71, 56, 243, 50, 92, 39, 0, 0, 0, 0, 0, 0, 0,
            ],
            merkle_root_hash: [
                73, 191, 174, 124, 249, 13, 196, 91, 111, 115, 181, 11, 47, 192, 113, 23, 162, 161,
                0, 149, 186, 157, 105, 31, 222, 228, 75, 53, 99, 96, 127, 188,
            ],
            timestamp: 1684365794,
            n_bits: 422038156,
            nonce: 4145232663,
            hash: [
                46, 225, 4, 199, 80, 124, 23, 73, 106, 229, 221, 69, 151, 96, 228, 69, 120, 93, 60,
                205, 129, 141, 96, 39, 26, 0, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
        };

        let block_header4 = BlockHeader {
            version: 551550976,
            prev_blockhash: [
                46, 225, 4, 199, 80, 124, 23, 73, 106, 229, 221, 69, 151, 96, 228, 69, 120, 93, 60,
                205, 129, 141, 96, 39, 26, 0, 0, 0, 0, 0, 0, 0,
            ],
            merkle_root_hash: [
                173, 134, 168, 102, 42, 4, 170, 107, 97, 155, 222, 70, 69, 53, 193, 3, 92, 175,
                183, 165, 75, 42, 48, 92, 220, 205, 175, 203, 143, 252, 111, 40,
            ],
            timestamp: 1684366299,
            n_bits: 422038156,
            nonce: 4106889822,
            hash: [
                8, 54, 29, 30, 190, 90, 168, 19, 242, 162, 14, 99, 74, 193, 46, 32, 39, 163, 75,
                151, 187, 10, 98, 60, 20, 0, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
        };

        let block_header5 = BlockHeader {
            version: 551550976,
            prev_blockhash: [
                8, 54, 29, 30, 190, 90, 168, 19, 242, 162, 14, 99, 74, 193, 46, 32, 39, 163, 75,
                151, 187, 10, 98, 60, 20, 0, 0, 0, 0, 0, 0, 0,
            ],
            merkle_root_hash: [
                39, 1, 237, 69, 75, 158, 237, 106, 99, 23, 172, 97, 247, 24, 204, 38, 108, 43, 89,
                255, 10, 141, 30, 148, 211, 194, 243, 185, 174, 48, 209, 53,
            ],
            timestamp: 1684367351,
            n_bits: 422038156,
            nonce: 99147372,
            hash: [
                165, 231, 179, 48, 219, 178, 234, 118, 41, 137, 248, 220, 243, 161, 77, 246, 161,
                109, 164, 82, 72, 24, 8, 179, 12, 0, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
        };

        let block_headers = vec![
            block_header1,
            block_header2,
            block_header3,
            block_header4,
            block_header5,
        ];
        let mut _utxo_set = UtxoSet::new_from_block_headers(block_headers)?;

        Ok(())
    }
}
