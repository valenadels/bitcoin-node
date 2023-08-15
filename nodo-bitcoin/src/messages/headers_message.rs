use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    net::TcpStream,
};

use crate::{
    block::block_hash::BlockHash,
    block_header::{block_header_bytes::BlockHeaderBytes, BlockHeader},
    compact_size::CompactSize,
    connectors::peer_connector::send_message,
    constants::{COMMAND_NAME_HEADERS, LENGTH_BLOCK_HEADERS, MAX_HEADERS_COUNT},
    header::Header,
    node_error::NodeError,
    utils::Utils,
};

use super::get_headers_message::GetHeadersMessage;

/// Represents a headers message that sends block headers to a node which previously
/// requested certain headers with a `getheaders` message.
///
/// A headers message can be empty.
///
/// # Fields
///
/// * `count` - Number of block headers up to a maximum of 2,000. Note: headers-first sync assumes
/// the sending node will send the maximum number of headers whenever possible.
///
/// * `headers` - Block headers: each 80-byte block header with an additional 0x00 suffixed.
///  This 0x00 is called the transaction count,
/// but because the headers message doesn’t include any transactions, the transaction count is
/// always zero.
#[derive(Debug)]
pub struct HeadersMessage {
    count: u64,
    headers: Vec<BlockHeaderBytes>,
}

impl HeadersMessage {
    /// Creates a new `headers_message`.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of block headers up to a maximum of 2,000.
    ///
    /// * `headers` - Block headers: each 80-byte block header
    pub fn new(count: u64, headers: Vec<BlockHeaderBytes>) -> Self {
        Self { count, headers }
    }

    /// Reads a varint that represents the headers count from a given TCP stream and returns a tuple with the value and the number of bytes read.
    ///
    /// # Arguments
    ///
    /// * `stream` - A reference to a `TcpStream` from which to read the varint.
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple with the value of the varint and the number of bytes read.
    ///
    /// # Errors
    ///
    /// This function may return an error if the input is not a valid varint, or if an error occurs while reading from the stream. The specific error types are defined in the `NodeError` enum.
    pub fn get_headers_count<R: Read>(source: &mut R) -> Result<u64, NodeError> {
        Ok(CompactSize::read_varint(source)?.get_value())
    }

    /// Reads a headers message from a given byte array and returns a `HeadersMessage`.
    /// Returns
    /// A `Result` containing a `HeadersMessage`.
    /// # Errors
    /// This function may return an error if the input is not a valid headers message
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NodeError> {
        let mut offset = 0;
        let (count, bytes_read) = Utils::read_varint(&bytes[offset..])?;
        offset += bytes_read;

        if count > MAX_HEADERS_COUNT {
            return Err(NodeError::InvalidSizeOfHeaders(
                "The count is greater than the maximum allowed".to_string(),
            ));
        }
        let mut headers = vec![];
        for _ in 0..count {
            let header = bytes[offset..(offset + LENGTH_BLOCK_HEADERS)].to_vec();
            headers.push(header);
            offset += LENGTH_BLOCK_HEADERS;
        }

        Ok(Self { count, headers })
    }
    /// Retrieves the block headers from the node.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `BlockHeader` instances on success,
    /// or a `NodeError` on failure.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError` if there was an error while converting the bytes to `BlockHeader`.
    pub fn retrieve_block_headers(&self) -> Result<Vec<BlockHeader>, NodeError> {
        let mut block_headers = Vec::with_capacity(self.count as usize);

        for header in &self.headers {
            let block_header = BlockHeader::from_bytes(header)?;
            block_headers.push(block_header);
        }

        Ok(block_headers)
    }
    ///returns the count of the headers
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Moves the block header file pointer to the block header with the specified starting header hash.
    ///
    /// # Arguments
    ///
    /// * `file` - A mutable reference to the opened file containing the block headers.
    /// * `buffer` - A mutable reference to the buffer used for reading block headers.
    /// * `starting_header_hash` - The starting header hash to search for.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the starting header is found and the cursor is moved to that position.
    fn move_file_to_starting_hash(
        file: &mut File,
        buffer: &mut BlockHeaderBytes,
        starting_header_hash: &BlockHash,
    ) -> Result<(), NodeError> {
        loop {
            match file.read_exact(buffer) {
                Ok(_) => {
                    let header = BlockHeader::from_bytes(buffer)?;

                    if header.hash() == starting_header_hash {
                        println!("Found starting hash");
                        break;
                    }
                }
                Err(_) => {
                    println!("Reached end of file, starting hash not found");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Saves headers after starting hash to the headers vector.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable array of 80 bytes used as a buffer for reading data.
    /// * `file` - A mutable reference to a `File` object representing the headers file.
    /// * `headers` - A mutable reference to a vector of vectors of bytes representing the headers.
    ///
    /// # Errors
    ///
    /// Returns an `Err` variant of `NodeError` if any error occurs during the process.
    ///
    /// The possible errors include:
    /// - `NodeError::FailedToRead` - If there was an error reading the headers file.
    fn obtain_headers_after_starting_hash(
        mut buffer: BlockHeaderBytes,
        file: &mut File,
        headers: &mut Vec<BlockHeaderBytes>,
    ) -> Result<(), NodeError> {
        for _ in 0..MAX_HEADERS_COUNT {
            match file.read_exact(&mut buffer) {
                Ok(_) => {
                    headers.push(buffer.to_vec());
                }
                Err(_) => {
                    println!("Finished sending headers");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Retrieves the following block headers starting from the specified header hash.
    ///
    /// # Arguments
    ///
    /// * `starting_header_hash` - The header hash to start retrieving block headers from.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Vec<u8>>)` - A vector containing the retrieved block headers as byte vectors.
    /// * `Err(NodeError)` - If there is an error opening or reading the block headers file.
    fn retrieve_following_headers(
        starting_header_hash: &BlockHash,
        file: &mut File,
    ) -> Result<Vec<BlockHeaderBytes>, NodeError> {
        let mut headers = Vec::new();
        let mut buffer = [0u8; LENGTH_BLOCK_HEADERS].to_vec();

        Self::move_file_to_starting_hash(file, &mut buffer, starting_header_hash)?;
        Self::obtain_headers_after_starting_hash(buffer, file, &mut headers)?;

        file.seek(SeekFrom::Current(-80))
            .map_err(|_| NodeError::FailedToRead("Failed to read headers file".to_string()))?;

        Ok(headers)
    }

    /// Sends a batch of block headers to the specified TCP stream starting from the given header hash.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a TCP stream representing the connection to the node.
    /// * `getheaders_message` - The getheaders message received from the node, used to determine the starting header hash.
    /// * `header_file` - A mutable reference to the file containing block headers for retrieval.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the headers message is successfully sent.
    /// * `Err(NodeError)` - If there is an error retrieving the block headers or sending the headers message.
    pub fn send_batch_headers(
        stream: &mut TcpStream,
        get_headers_message: GetHeadersMessage,
        file: &mut File,
    ) -> Result<(), NodeError> {
        let starting_header_hash = get_headers_message.header_hashes[0];

        let headers_to_send = Self::retrieve_following_headers(&starting_header_hash, file)?;

        println!("Sending {:?} headers", headers_to_send.len());

        let headers_message = HeadersMessage::new(headers_to_send.len() as u64, headers_to_send);

        headers_message.send(stream)?;

        Ok(())
    }

    /// Converts the `HeadersMessage` to a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(CompactSize::new(self.count as usize).to_bytes());
        for header in &self.headers {
            bytes.extend(header);
            bytes.extend(vec![0u8; 1]);
        }
        bytes
    }

    /// Sends a Headers message over a TCP stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a `TcpStream` object representing the TCP connection.
    ///
    /// # Errors
    ///
    /// Returns an `Err` variant of `NodeError` if any error occurs during the process.
    ///
    /// The possible errors include:
    /// - `NodeError::FailedToSend` - If there was an error sending the message over the TCP stream.
    ///
    pub fn send(&self, stream: &mut TcpStream) -> Result<(), NodeError> {
        let headers_message = self.to_bytes();
        let header = Header::create_header(&headers_message, COMMAND_NAME_HEADERS)?;

        let mut bytes = vec![];
        bytes.extend(header);
        bytes.extend(headers_message);

        send_message(stream, bytes)
    }
}

#[cfg(test)]
mod tests {
    use crate::{messages::headers_message::HeadersMessage, node_error::NodeError};

    #[test]
    fn test_headers_message_is_deserialized() -> Result<(), NodeError> {
        let headers_message: [u8; 82] = [
            1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 41, 171, 95, 73, 255, 255, 0, 29, 29, 172, 43, 124, 0,
        ];
        let deserialized_headers_message = HeadersMessage::from_bytes(&headers_message)?;

        let block_header: [u8; 80] = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 41, 171, 95, 73, 255, 255, 0, 29, 29, 172, 43, 124,
        ];

        assert_eq!(deserialized_headers_message.count, 1);
        assert_eq!(
            deserialized_headers_message.headers[0],
            block_header.to_vec()
        );
        Ok(())
    }
}
