use std::{fs, net::TcpStream};

use bitcoin_hashes::{sha256d, Hash};

use crate::{
    block::block_hash::BlockHash,
    config::obtain_dir_path,
    connectors::peer_connector::send_message,
    constants::{COMMAND_NAME_BLOCK, PATH_BLOCKS},
    header::Header,
    node_error::NodeError,
    utils::Utils,
};

use super::get_data_message::GetDataMessage;

/// A message containing a block header and a list of transactions.
#[derive(Debug)]
pub struct BlockMessage {
    /// The block header, which is an 80-byte array.
    _block_header: [u8; 80],

    /// The number of transactions in the block.
    _txn_count: u64,

    /// The list of transactions in the block
    _txns: Vec<u8>,
}

impl BlockMessage {
    // Creates a new `BlockMessage` instance with the given block header, transaction count, and
    /// list of transactions.
    ///
    /// # Arguments
    ///
    /// * `block_header` - The block header, which is an 80-byte array.
    /// * `txn_count` - The number of transactions in the block.
    /// * `txns` - The list of transactions in the block
    pub fn new(block_header: [u8; 80], txn_count: u64, txns: Vec<u8>) -> Self {
        Self {
            _block_header: block_header,
            _txn_count: txn_count,
            _txns: txns,
        }
    }
    /// Deserializes a `BlockMessage` from a byte array.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The byte array to deserialize from.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToRead` error if the byte array is invalid or cannot be read.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NodeError> {
        let mut block_header = [0u8; 80];
        block_header.copy_from_slice(&bytes[0..80]);

        let mut offset = 80;

        let (txn_count, bytes_read) = Utils::read_varint(&bytes[offset..])?;
        offset += bytes_read;

        let txns = bytes[offset..].to_vec();
        Ok(Self {
            _block_header: block_header,
            _txn_count: txn_count,
            _txns: txns,
        })
    }
    /// Sends a BlockMessage to the provided TcpStream.
    ///
    /// This function takes a mutable reference to a TcpStream and a GetDataMessage as input,
    /// reads the block hash from the GetDataMessage, finds the corresponding block path,
    /// reads the block data from the file, creates a header for the block,
    /// and then sends the header and block data as a message over the TcpStream.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to a TcpStream where the message will be sent.
    /// * `getdata_message` - The GetDataMessage to look for the block.
    ///
    /// # Errors
    ///
    /// This function may return a NodeError in the following cases:
    /// * If the block hash cannot be retrieved from the GetDataMessage.
    /// * If the block path cannot be found for the given block hash.
    /// * If there is a failure to read the block file.
    /// * If there is an error while sending the message over the TcpStream.
    ///
    pub fn send_message(
        stream: &mut TcpStream,
        getdata_message: GetDataMessage,
    ) -> Result<(), NodeError> {
        let block_hash = getdata_message.block_hash()?;
        let block_path = match Self::block_path(block_hash) {
            Some(path) => path,
            None => {
                return Err(NodeError::FailedToRead(
                    "Failed to get block path".to_string(),
                ))
            }
        };

        let block_bytes = fs::read(block_path)
            .map_err(|_| NodeError::FailedToRead("Failed to read block file".to_string()))?;

        let header = Header::create_header(&block_bytes, COMMAND_NAME_BLOCK)?;

        let mut bytes = vec![];

        bytes.extend(&header);
        bytes.extend(&block_bytes);

        send_message(stream, bytes)
    }

    /// Returns the file path for the block with the specified hash. The file name is based on the
    /// hash of the block and has the `.bin` extension.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - A fixed size array of bytes representing the hash of the block.
    ///
    /// # Returns
    ///
    /// An `Option` containing the file path for the block if the hash could be converted to a string,
    /// otherwise returns `None`.
    pub fn block_path(block_hash: &BlockHash) -> Option<String> {
        let hash_string = match sha256d::Hash::from_slice(block_hash) {
            Ok(hash) => hash,
            Err(_) => {
                println!("Error in block's hash");
                return None;
            }
        };

        let directory = match obtain_dir_path(PATH_BLOCKS.to_owned()) {
            Ok(value) => value,
            Err(_) => return None,
        };

        let path = format!("{}/{}.bin", directory, hash_string);

        if let Err(e) = fs::create_dir_all(&directory) {
            println!("Error creating directory 'blocks': {}", e);
            return None;
        }

        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_block_message() {
        let bytes = [
            // Block header 80 bytes
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a,
            0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
            0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
            // Transaction count (1 byte)
            0x02, // Transaction #1 (4 bytes size + 4 bytes data)
            0x01, 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03,
            // Transaction #2 (1 byte size + 1 byte data)
            0x01, 0x01,
        ];

        let block_message = BlockMessage::from_bytes(&bytes).unwrap();

        assert_eq!(
            block_message._block_header,
            [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a,
                0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
                0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
                0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50
            ]
        );

        assert_eq!(block_message._txn_count, 2);
        assert_eq!(
            block_message._txns,
            vec![0x01, 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x01, 0x01]
        );
    }
}
