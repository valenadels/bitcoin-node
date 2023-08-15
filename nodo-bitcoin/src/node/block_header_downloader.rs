use std::{
    fs::File,
    io::{self, Seek, Write},
    net::TcpStream,
};

use crate::{
    block_header::{block_header_bytes::BlockHeaderBytes, BlockHeader},
    config::obtain_dir_path,
    connectors::peer_connector::receive_message,
    constants::{BLOCK_HEADERS_FILE, LENGTH_BLOCK_HEADERS, MAX_HEADERS_COUNT},
    header::Header,
    logger::Logger,
    messages::{get_headers_message::GetHeadersMessage, headers_message::HeadersMessage},
    node::{
        message_type::MessageType,
        receive_messages::{
            receive_addr_message, receive_feefilter_message, receive_inv_message, send_pong_message,
        },
    },
    node_error::NodeError,
    ui::ui_message::UIMessage,
    utils::Utils,
};

use super::read::read_last_block_header;

/// The `BlockHeaderDownloader` struct handles the connection to a peer and the downloading of block headers.
pub struct BlockHeaderDownloader<'a> {
    /// A mutable reference to a TcpStream that represents the connection to the node.
    stream: &'a mut TcpStream,
    /// A File to write the block headers to.
    file: File,
}

impl<'a> BlockHeaderDownloader<'a> {
    /// Creates a new BlockHeaderDownloader.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a TcpStream that represents the connection to the node.
    ///
    /// # Returns
    ///
    /// A new BlockHeaderDownloader with the given TcpStream and a File to write the block headers to.
    ///
    /// # Errors
    ///
    /// If the file cannot be opened, a NodeError is returned.
    pub fn new(stream: &'a mut TcpStream) -> Result<Self, NodeError> {
        let dir_headers_file = obtain_dir_path(BLOCK_HEADERS_FILE.to_owned())?;
        let file = File::options()
            .write(true)
            .create(true)
            .open(dir_headers_file)
            .map_err(|_| {
                NodeError::FailedToOpenFile("Failed to open block headers file".to_string())
            })?;
        if Utils::is_tcpstream_connected(stream) {
            println!(
                "Tcp stream {:?} is connected to download headers",
                stream.peer_addr()
            );
            Ok(Self { stream, file })
        } else {
            println!(
                "Tcp stream {:?} is not connected to download headers, retrying...",
                stream.peer_addr()
            );
            Err(NodeError::TcpStreamNotConnected(
                "Tcp stream is not connected".to_string(),
            ))
        }
    }

    /// Handles the connection to a peer.
    /// The function sends a "getheaders" message to the peer and then waits for the peer
    /// to respond with a "headers" message. The function writes the block headers from the "headers" message into a file to be stored.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` representing the connection
    ///             to the peer that is expected to send the `headers` message.
    /// * `logger` - A mutable reference to a `Logger` to log messages received from the peer.
    /// * `ui_sender` - A mutable reference to a `glib::Sender<UIMessage>` to send messages to the UI.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - An empty `Ok` if the function completes successfully.
    ///
    /// # Errors
    ///
    /// * `NodeError` - A `NodeError` is returned if there is an error sending or receiving
    ///                the messages.
    pub fn start(
        &mut self,
        logger: &Logger,
        ui_sender: &glib::Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        println!("Sending getheaders message: Starting Headers download");
        let last_block_header = read_last_block_header()?;
        GetHeadersMessage::send_message(self.stream, &last_block_header)?;

        self.handle_download(logger, ui_sender, last_block_header)
    }

    /// Handles the initial block headers received over a TCP stream and performs corresponding actions.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` to receive messages from.
    /// * `logger` - A mutable reference to a `Logger` to log messages received from the peer.
    /// * `ui_sender` - A mutable reference to a `glib::Sender<UIMessage>` to send messages to the UI.
    /// * `last_bh` - A vector of bytes representing the last block header in the block headers file.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the initial block headers are successfully handled,
    /// or a `NodeError` on failure.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if there was an error while receiving or handling messages,
    /// or if the maximum number of headers is reached.
    fn handle_download(
        &mut self,
        logger: &Logger,
        ui_sender: &glib::Sender<UIMessage>,
        mut last_bh: BlockHeaderBytes,
    ) -> Result<(), NodeError> {
        let mut count_headers = 1;
        loop {
            let mut header = Header::new(self.stream)?;
            let command_name = header.extract_command_name()?;
            if command_name == MessageType::Headers {
                println!(
                    "Receiving command: {:?}, count: {}",
                    command_name, count_headers
                );
                count_headers += 1;
                ui_sender
                    .send(UIMessage::UpdateHeadersProgress)
                    .map_err(|_| {
                        NodeError::FailedToSendMessage(
                            "Failed to send headers count message to UI".to_string(),
                        )
                    })?;
            } else {
                println!("Receiving command: {:?}", command_name);
            }
            logger.log(format!("Received: {:?} in headers download", command_name))?;

            match command_name {
                MessageType::Headers => {
                    if self.receive_headers_message(logger, last_bh)? == MAX_HEADERS_COUNT {
                        last_bh = read_last_block_header()?;

                        GetHeadersMessage::send_message(self.stream, &last_bh)?;
                        continue;
                    } else {
                        ui_sender
                            .send(UIMessage::HeadersDownloadFinished)
                            .map_err(|_| {
                                NodeError::FailedToSendMessage(
                                    "Failed to send headers download finished message to UI"
                                        .to_string(),
                                )
                            })?;
                        break;
                    }
                }
                MessageType::Ping => {
                    println!("Handle ping message");
                    send_pong_message(self.stream, &header)?;
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
                    receive_addr_message(self.stream, &header)?;
                    continue;
                }
                MessageType::FeeFilter => {
                    println!("Recieved a feefilter message");
                    receive_feefilter_message(self.stream, &header)?;
                    continue;
                }
                MessageType::Inv => {
                    println!("Recieved a inv message");
                    receive_inv_message(self.stream, &header)?;
                    continue;
                }
                _ => {
                    println!("Command not supported");
                    receive_message(self.stream, header.payload_size())?;
                }
            }
        }
        Ok(())
    }

    /// Receives the block headers from the "headers" message,
    /// adds them to the node's block header chain, and returns a touple containing the headers count and the block headers.
    ///
    /// # Arguments
    /// * `logger` - A mutable reference to a `Logger` to log header.hash received from the peer.
    /// * `last_bh` - A vector of bytes representing the last block header in the block headers file.
    ///
    /// # Returns
    /// A `Result` containing a touple of the headers count and the block headers if the function succeeds.
    ///
    /// # Errors
    ///
    /// The function returns a `NodeError` if there is an error while reading or deserializing
    /// the headers message from the stream.
    pub fn receive_headers_message(
        &mut self,
        logger: &Logger,
        last_bh: BlockHeaderBytes,
    ) -> Result<u64, NodeError> {
        let mut block_headers = Vec::new();
        let headers_count = HeadersMessage::get_headers_count(self.stream)?;

        for _ in 0..headers_count {
            let recv_block_header = receive_message(self.stream, LENGTH_BLOCK_HEADERS)?;
            logger.log("Downloaded new Header".to_string())?;
            receive_message(self.stream, 1)?;
            block_headers.push(recv_block_header);
        }

        let mut last_block_headers = Vec::new();
        let last_bh_timestamp = BlockHeader::from_bytes(&last_bh)?.timestamp;

        if headers_count < MAX_HEADERS_COUNT {
            for b in block_headers.iter() {
                let block_header = BlockHeader::from_bytes(b)?;
                if block_header.timestamp >= last_bh_timestamp {
                    last_block_headers.push(b.to_vec());
                }
            }
            self.write_block_headers_to_file(&last_block_headers)?;
        } else {
            self.write_block_headers_to_file(&block_headers)?;
        };

        Ok(headers_count)
    }

    /// Writes the block headers to the block headers file.
    ///
    /// # Arguments
    ///
    /// * `block_headers` - A vector of vectors of bytes representing the block headers to write to the file.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `()` if the function succeeds.
    ///
    /// # Errors
    ///
    /// The function returns a `NodeError` if there is an error opening or writing to the file.
    pub fn write_block_headers_to_file(
        &self,
        block_headers: &Vec<BlockHeaderBytes>,
    ) -> Result<(), NodeError> {
        let mut file = &self.file;
        let file_size = file.seek(io::SeekFrom::End(0)).map_err(|_| {
            NodeError::FailedToWriteAll("Failed to write block header to file".to_string())
        })?;

        file.seek(io::SeekFrom::Start(file_size)).map_err(|_| {
            NodeError::FailedToWriteAll("Failed to write block header to file".to_string())
        })?;

        for header in block_headers {
            file.write_all(header).map_err(|_| {
                NodeError::FailedToWriteAll("Failed to write block header to file".to_string())
            })?;
        }

        Ok(())
    }
}
