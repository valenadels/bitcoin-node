use std::io::Read;

use crate::{
    block::tx_hash::TxHash,
    compact_size::CompactSize,
    connectors::peer_connector::receive_message,
    constants::{LENGTH_HEIGHT, LENGTH_SEQUENCE},
    node_error::NodeError,
};

use super::outpoint::Outpoint;

#[derive(Debug, Clone)]
/// Represents an input for a transaction.
pub struct TxInput {
    /// The previous transaction output being spent.
    pub previous_output: Outpoint,
    /// The length of the signature script in bytes.
    pub script_bytes: CompactSize,
    /// The signature script that provides the unlocking script for spending the previous output.
    pub signature_script: Vec<u8>,
    /// The sequence number of the input.
    pub sequence: u32,
}

impl TxInput {
    /// Reads a transaction input from a reader.
    ///
    /// # Arguments
    ///
    /// * `block` - A mutable reference to a reader implementing the `Read` trait, can be a file, TcpStream, etc.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `TxInput` if successful, or a `NodeError` if an error occurs.
    ///
    /// # Errors
    ///
    /// If the previous output is not found, or if the sequence is not found, a `NodeError` will be returned.
    pub fn read_tx_input<R: Read>(block: &mut R) -> Result<TxInput, NodeError> {
        let previous_output = Outpoint::read_outpoint(block)?;

        let script_bytes = CompactSize::read_varint(block)?;
        let count = script_bytes.get_value();
        let signature_script = receive_message(block, count as usize)?;

        let sequence = receive_message(block, LENGTH_SEQUENCE)?;

        if sequence.len() == LENGTH_SEQUENCE {
            let new_sequence =
                u32::from_le_bytes([sequence[0], sequence[1], sequence[2], sequence[3]]);

            let tx_input = TxInput {
                previous_output,
                script_bytes,
                signature_script,
                sequence: new_sequence,
            };

            Ok(tx_input)
        } else {
            Err(NodeError::FailedToCreateTxInput(
                "Failed to create tx input".to_string(),
            ))
        }
    }

    /// Retrieves the coinbase transaction input from a block.
    ///
    /// # Arguments
    ///
    /// * `block` - A mutable reference to a reader implementing the `Read` trait, such as a file or a `TcpStream`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the parsed `TxInput` if successful, or a `NodeError` if an error occurs.
    pub fn read_tx_coinbase_input<R: Read>(block: &mut R) -> Result<TxInput, NodeError> {
        let previous_output = Outpoint::read_outpoint(block)?;

        let script_bytes = CompactSize::read_varint(block)?;
        let count = script_bytes.get_value();

        let mut height = receive_message(block, LENGTH_HEIGHT)?;

        let _block_height = height[0..LENGTH_HEIGHT].to_vec();
        let signature_script = receive_message(block, count as usize - 4)?;

        height.extend_from_slice(&signature_script);

        let sequence = receive_message(block, LENGTH_SEQUENCE)?;

        if sequence.len() == LENGTH_SEQUENCE {
            let new_sequence =
                u32::from_le_bytes([sequence[0], sequence[1], sequence[2], sequence[3]]);

            let tx_input = TxInput {
                previous_output,
                script_bytes,
                signature_script: height,
                sequence: new_sequence,
            };

            Ok(tx_input)
        } else {
            Err(NodeError::FailedToCreateTxInput(
                "Failed to create coinbase tx input".to_string(),
            ))
        }
    }

    /// Converts a `TxInput` to a byte vector.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the byte representation of the `TxInput`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.previous_output.to_bytes());
        bytes.extend(self.script_bytes.to_bytes());
        bytes.extend(self.signature_script.to_vec());
        bytes.extend(self.sequence.to_le_bytes().to_vec());
        bytes
    }

    /// Creates a new unsigned transaction input.
    pub fn new_unsigned(tx_id: &TxHash, index: &u32, previous_pk_script: &[u8]) -> TxInput {
        let previous_output = Outpoint {
            tx_id: tx_id.to_owned(),
            index: *index,
        };

        let signature_script = previous_pk_script.to_vec();
        let script_bytes = CompactSize::new(signature_script.len());
        let sequence = 0xffffffff;

        TxInput {
            previous_output,
            script_bytes,
            signature_script,
            sequence,
        }
    }
}
