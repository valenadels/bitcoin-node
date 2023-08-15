use std::io::Read;

use crate::{
    block::tx_hash::TxHash,
    compact_size::CompactSize,
    connectors::peer_connector::receive_message,
    constants::{LENGTH_VALUE, SATOSHI_CONVERSION_COEFFICIENT},
    node_error::NodeError,
    wallet::bitcoin_address::BitcoinAddress,
};

use super::pk_script::PkScript;

#[derive(Debug, Clone)]
/// Represents an output for a transaction.
pub struct TxOutput {
    /// The value of the output in satoshis.
    pub value: i64,
    /// The length of the public key script in bytes.
    pub pk_script_bytes: CompactSize,
    /// The public key script that controls spending of the output.
    pub pk_script: PkScript,
    /// The transaction id of the transaction that created the output.
    pub tx_id: TxHash,
    /// The index of the output in the transaction.
    pub index: u64,
    /// The path of the block that contains the output.
    pub block_path: String,
}

impl TxOutput {
    /// Reads a transaction output from a reader.
    ///
    /// # Arguments
    ///
    /// * `block` - A mutable reference to a reader implementing the `Read` trait, can be a file, TcpStream, etc.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `TxOutput` if successful, or a `NodeError` if an error occurs.
    ///
    /// # Errors
    ///
    /// If the `TxOutput` is not valid, a `NodeError` is returned.
    pub fn read_tx_output_from_block<R: Read>(
        block: &mut R,
        index: u64,
    ) -> Result<TxOutput, NodeError> {
        let value_vec = receive_message(block, LENGTH_VALUE)?;
        let value_in_satoshis = i64::from_le_bytes(value_vec.try_into().map_err(|_| {
            NodeError::FailedToParse("Failed to convert Vec<u8> to [u8;8]".to_string())
        })?);

        let pk_script_bytes = CompactSize::read_varint(block)?;
        let pk_script = receive_message(block, pk_script_bytes.get_value() as usize)?;

        Ok(TxOutput {
            value: value_in_satoshis,
            pk_script_bytes,
            pk_script,
            index,
            tx_id: Vec::new(),
            block_path: String::new(),
        })
    }

    /// Converts a `TxOutput` to a byte vector.
    ///
    /// # Returns
    ///
    /// A byte vector containing the `TxOutput`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.value.to_le_bytes().to_vec());
        bytes.extend(self.pk_script_bytes.to_bytes());
        bytes.extend(self.pk_script.to_vec());

        bytes
    }

    /// Returns the value of the output in tBC.
    pub fn value(&self) -> f64 {
        self.value as f64 / SATOSHI_CONVERSION_COEFFICIENT
    }

    /// Creates a new `TxOutput`.
    pub fn new(value: f64, pk_script: PkScript, index: u64) -> TxOutput {
        let value_in_satoshis = (value * SATOSHI_CONVERSION_COEFFICIENT) as i64;
        TxOutput {
            value: value_in_satoshis,
            pk_script_bytes: CompactSize::new(pk_script.len()),
            pk_script,
            index,
            tx_id: Vec::new(),
            block_path: String::new(),
        }
    }

    /// Checks if the output contains the bitcoin address.
    pub fn contains_address(&self, address: &BitcoinAddress) -> bool {
        let address_pk_script = BitcoinAddress::to_pk_script(address);

        self.pk_script == address_pk_script
    }

    /// Returns the public key scripts of the given transaction outputs.
    pub fn pk_scripts(tx_outputs: &[&TxOutput]) -> Vec<PkScript> {
        tx_outputs
            .iter()
            .map(|tx_output| tx_output.pk_script.clone())
            .collect()
    }
}
