use crate::{
    block::tx_hash::TxHash, connectors::peer_connector::receive_message, constants::LENGTH_INDEX,
    node_error::NodeError,
};
use std::io::Read;

#[derive(Debug, Clone)]
/// Represents an outpoint.
pub struct Outpoint {
    /// The hash of the transaction of the output being spent.
    pub tx_id: TxHash,
    /// The index of the output within the transaction.
    pub index: u32,
}

impl Outpoint {
    /// Reads an outpoint from a reader.
    ///
    /// # Arguments
    ///
    /// * `block` - A mutable reference to a reader implementing the `Read` trait, can be a file, TcpStream, etc.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `Outpoint` if successful, or a `NodeError` if an error occurs.
    ///
    /// # Errors
    ///
    /// If the `Outpoint` is not valid, a `NodeError` is returned.
    pub fn read_outpoint<R: Read>(block: &mut R) -> Result<Outpoint, NodeError> {
        let hash = receive_message(block, 32)?;
        let index = receive_message(block, 4)?;
        if index.len() == LENGTH_INDEX {
            let new_index = u32::from_le_bytes([index[0], index[1], index[2], index[3]]);

            return Ok(Outpoint {
                tx_id: hash,
                index: new_index,
            });
        }

        Err(NodeError::FailedToCreateOutpoint(
            "Failed to create outpint".to_string(),
        ))
    }

    /// Converts an `Outpoint` to a byte vector.
    ///
    /// # Returns
    ///
    /// A byte vector containing the `Outpoint`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.tx_id.to_vec());
        bytes.extend(self.index.to_le_bytes().to_vec());
        bytes
    }
}
