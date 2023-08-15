use std::{io::Read, net::TcpStream};

use bitcoin_hashes::{sha256d, Hash};

use crate::{
    block::block_hash::BlockHash, block_header::block_header_bytes::BlockHeaderBytes,
    compact_size::CompactSize, connectors::peer_connector::send_message,
    constants::COMMAND_NAME_GETHEADERS, header::Header, node::read::retrieve_version,
    node_error::NodeError,
};

/// Represents a "GetHeaders" message that requests block headers from a node.
#[derive(Debug)]
pub struct GetHeadersMessage {
    /// The protocol version of the transmitting node.
    pub protocol_version: i32,
    /// The number of header hashes included in the message.
    pub hash_count: CompactSize,
    /// The hashes of the block headers being requested.
    pub header_hashes: Vec<BlockHash>,
}

impl GetHeadersMessage {
    /// Returns the byte sequence representing a getheaders message for the purpose of fetching all the headers
    /// contained in the node to which the current node is connected.
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the getheaders message.
    pub fn to_bytes(block_header: &[u8]) -> Vec<u8> {
        let protocol_version: i32 = retrieve_version();
        let hash_count: u8 = 1;
        let last_header_hash = sha256d::Hash::hash(block_header).to_byte_array().to_vec();

        let hash_stop = [0; 32];

        let mut bytes = vec![];
        bytes.extend(protocol_version.to_le_bytes());
        bytes.extend(hash_count.to_le_bytes());
        bytes.extend(last_header_hash);
        bytes.extend(&hash_stop);

        bytes
    }

    /// Reads a "GetHeaders" message from a TCP stream and constructs a `GetheadersMessage`.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a TCP stream from which to read the message.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `GetheadersMessage` if successful, or a `NodeError` if an error occurs.
    pub fn from_stream(stream: &mut TcpStream) -> Result<GetHeadersMessage, NodeError> {
        let mut buffer_protocol_version = [0; 4];
        stream
            .read_exact(&mut buffer_protocol_version)
            .map_err(|_| NodeError::FailedToRead("Failed to read all bytes".to_string()))?;

        let protocol_version = i32::from_le_bytes(buffer_protocol_version);
        println!("Protocol version: {}", protocol_version);

        let hash_count = CompactSize::read_varint(stream)?;
        println!("Hash count: {}", hash_count.get_value());

        let mut header_hashes = vec![];

        let mut hash = [0; 32];
        stream
            .read_exact(&mut hash)
            .map_err(|_| NodeError::FailedToRead("Failed to read all bytes".to_string()))?;

        header_hashes.push(hash);

        stream
            .read_exact(&mut hash)
            .map_err(|_| NodeError::FailedToRead("Failed to read all bytes".to_string()))?;

        Ok(GetHeadersMessage {
            protocol_version,
            hash_count,
            header_hashes,
        })
    }

    /// Sends a getheaders message to the node to which the current node is connected.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a TcpStream that represents the connection to the node.
    /// * `block_header` - A vector of bytes representing the last block header hash that the node has.
    ///
    /// # Returns
    ///
    /// An empty Result.
    ///
    /// # Errors
    ///
    /// If the message cannot be sent or the header cannot be created, a NodeError is returned.
    pub fn send_message(
        stream: &mut TcpStream,
        block_header: &BlockHeaderBytes,
    ) -> Result<(), NodeError> {
        let getheaders_message = GetHeadersMessage::to_bytes(block_header);
        let header_getheaders = Header::create_header(&getheaders_message, COMMAND_NAME_GETHEADERS);

        let header = match header_getheaders {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        let mut bytes = vec![];
        bytes.extend(header);
        bytes.extend(getheaders_message);

        send_message(stream, bytes)
    }
}

#[cfg(test)]
mod test {
    use bitcoin_hashes::{sha256d, Hash};

    use crate::{
        block_header::GENESIS_BLOCK_HEADER, messages::get_headers_message::GetHeadersMessage,
        node::read::retrieve_version, node_error::NodeError,
    };

    #[test]
    fn test_getheader_creation() -> Result<(), NodeError> {
        let block_headers = GENESIS_BLOCK_HEADER.to_bytes();
        let getheaders_message = GetHeadersMessage::to_bytes(&block_headers);
        let version = retrieve_version();
        let hash_count: u8 = 1;
        let mut expected_getheaders_message = Vec::new();
        expected_getheaders_message.extend(version.to_le_bytes());
        expected_getheaders_message.extend(hash_count.to_le_bytes());
        expected_getheaders_message.extend(
            sha256d::Hash::hash(&GENESIS_BLOCK_HEADER.to_bytes())
                .to_byte_array()
                .to_vec(),
        );
        expected_getheaders_message.extend([0; 32].to_vec());

        assert_eq!(getheaders_message, expected_getheaders_message);

        Ok(())
    }
}
