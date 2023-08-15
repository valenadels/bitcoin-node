use super::tx_output::TxOutput;
use crate::{
    block::{retrieve_transactions_from_block, tx_hash::TxHash},
    block_header::BlockHeader,
    messages::block_message::BlockMessage,
    node_error::NodeError,
    wallet::{account::Account, bitcoin_address::BitcoinAddress},
};
use std::collections::HashMap;
#[derive(Debug, Clone)]
/// Represents the Unspent Transaction Outputs (UTXO) set.
pub struct UtxoSet {
    /// The data structure that stores the UTXO set, transaction IDs are mapped to their associated transaction outputs as a key-value pair.
    pub set: HashMap<TxHash, Vec<TxOutput>>,
}
impl UtxoSet {
    /// Updates the UTXO set by processing the transactions in a block.
    ///
    /// # Arguments
    ///
    /// * `utxo_set` - A mutable reference to the UTXO set represented as a HashMap with transaction IDs as keys and associated transaction outputs as values.
    /// * `block_header` - A reference to the BlockHeader object representing the block containing the transactions to be processed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the UTXO set is successfully updated, or an error of type `NodeError` if there was a problem retrieving the block transactions.
    pub fn update(&mut self, block_path: &String) -> Result<(), NodeError> {
        println!("Updating UTXO set from block: {:?}", block_path);
        let transactions = retrieve_transactions_from_block(block_path)?;
        for mut transaction in transactions {
            transaction.add_block_path_to_tx_outs(block_path);

            let tx_outputs = transaction.tx_outputs();
            let tx_id = transaction.tx_id();
            for tx_input in transaction.tx_inputs() {
                let outpoint = &tx_input.previous_output;
                if self.contains_key(&outpoint.tx_id) {
                    if let Some(tx_outputs) = self.tx_outputs(&outpoint.tx_id) {
                        for i in 0..tx_outputs.len() {
                            if tx_outputs[i].index == outpoint.index as u64 {
                                tx_outputs.remove(i);
                                if tx_outputs.is_empty() {
                                    self.remove(&outpoint.tx_id);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            self.insert(tx_id, tx_outputs.clone());
        }
        Ok(())
    }
    /// Inserts transaction outputs associated with a specific Bitcoin address into the Node's set.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to the Node object.
    /// * `tx_id` - The transaction ID as a vector of bytes.
    /// * `tx_outputs` - The vector of `TxOutput` objects to be inserted.
    /// * `address` - The Bitcoin address to check for inclusion in the transaction outputs.
    pub fn insert_for_account(
        &mut self,
        tx_id: TxHash,
        tx_outputs: Vec<TxOutput>,
        address: &BitcoinAddress,
    ) {
        for tx_output in tx_outputs {
            if tx_output.contains_address(address) {
                let outputs = self.set.entry(tx_id.clone()).or_insert_with(Vec::new);
                outputs.push(tx_output);
            }
        }
    }
    /// Updates the UTXO (Unspent Transaction Outputs) set for a specific Bitcoin address
    /// based on the transactions retrieved from a block.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to the Node object.
    /// * `block_path` - The path to the block from which to retrieve transactions, as a string.
    /// * `address` - The Bitcoin address associated with the UTXO set to be updated.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success (`Ok`) if the UTXO set was updated successfully,
    /// or a `NodeError` if any error occurred during the update process.
    pub fn update_for_account(
        &mut self,
        block_path: &String,
        address: &BitcoinAddress,
    ) -> Result<(), NodeError> {
        println!(
            "Updating UTXO set from block: {:?} for account: {:?}",
            block_path,
            address.bs58_to_string()
        );
        let transactions = retrieve_transactions_from_block(block_path)?;
        for mut transaction in transactions {
            transaction.add_block_path_to_tx_outs(block_path);

            let tx_outputs = transaction.tx_outputs();
            let tx_id = transaction.tx_id();
            for tx_input in transaction.tx_inputs() {
                let outpoint = &tx_input.previous_output;
                if self.contains_key(&outpoint.tx_id) {
                    if let Some(tx_outputs) = self.tx_outputs(&outpoint.tx_id) {
                        for i in 0..tx_outputs.len() {
                            if tx_outputs[i].index == outpoint.index as u64 {
                                tx_outputs.remove(i);
                                if tx_outputs.is_empty() {
                                    self.remove(&outpoint.tx_id);
                                }
                                break;
                            }
                        }
                    }
                }
            }
            self.insert_for_account(tx_id, tx_outputs.clone(), address);
        }
        Ok(())
    }
    /// Creates the UTXO (Unspent Transaction Outputs) set from a list of block headers.
    ///
    /// # Arguments
    ///
    /// * `block_headers` - A vector of BlockHeader representing the block headers from which to retrieve the UTXO set.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the UTXO set as a HashMap with transaction IDs as keys and associated transaction outputs as values if successful, or an error of type `NodeError` if there was a problem updating the UTXO set.
    pub fn new_from_block_headers(block_headers: Vec<BlockHeader>) -> Result<UtxoSet, NodeError> {
        let mut utxo_set = UtxoSet::new();
        for block_header in block_headers.iter() {
            let block_hash = block_header.hash().as_slice().try_into().map_err(|_| {
                NodeError::FailedToParse("Failed to convert block hash to array".to_string())
            })?;
            let block_path = match BlockMessage::block_path(block_hash) {
                Some(block_path) => block_path,
                None => {
                    return Err(NodeError::FailedToRead(
                        "Failed to get block path".to_string(),
                    ))
                }
            };
            match utxo_set.update(&block_path) {
                Ok(_) => (),
                Err(_) => {
                    println!("UTXO set was not updated because block isn't downloaded");
                }
            }
        }
        Ok(utxo_set)
    }
    /// Creates an empty UTXO set
    pub fn new() -> UtxoSet {
        UtxoSet {
            set: HashMap::new(),
        }
    }
    /// Checks if the UTXO set contains a transaction ID.
    pub fn contains_key(&self, tx_id: &TxHash) -> bool {
        self.set.contains_key(tx_id)
    }
    /// Gets the transaction outputs associated with a transaction ID.
    pub fn tx_outputs(&mut self, tx_id: &TxHash) -> Option<&mut Vec<TxOutput>> {
        self.set.get_mut(tx_id)
    }
    /// Inserts a transaction ID and associated transaction outputs into the UTXO set.
    pub fn insert(&mut self, tx_id: TxHash, tx_outputs: Vec<TxOutput>) {
        self.set.insert(tx_id, tx_outputs);
    }
    /// Removes a transaction ID and associated transaction outputs from the UTXO set.
    pub fn remove(&mut self, tx_id: &TxHash) {
        self.set.remove(tx_id);
    }
    /// Gets the UTXO set for a given Bitcoin address.
    ///
    /// # Arguments
    ///
    /// * `bitcoin_address` - A reference to a vector of bytes representing the Bitcoin address.
    ///
    /// # Returns
    ///
    /// Returns a UtxoSet containing the UTXOs for the given Bitcoin address.
    pub fn users_utxo_set(&self, users_pk_hash: &Vec<u8>) -> UtxoSet {
        let mut users_utxo_set = UtxoSet::new();
        println!("Creating the users UTXO set...");
        for tx_tuple in self.set.iter() {
            let tx_outputs = tx_tuple.1;
            let mut users_tx_outputs = Vec::new();
            for tx_output in tx_outputs {
                let tx_output_pk_hash = match Account::pk_script_to_pk_hash(&tx_output.pk_script) {
                    Ok(pk_hash) => pk_hash,
                    Err(_) => continue,
                };
                if &tx_output_pk_hash == users_pk_hash {
                    users_tx_outputs.push(tx_output.clone());
                }
            }
            if !users_tx_outputs.is_empty() {
                users_utxo_set.insert(tx_tuple.0.clone(), users_tx_outputs);
            }
        }
        users_utxo_set
    }

    /// Gets the sum of the UTXOs that can be spent.
    pub fn sum_of_outs(tx_outs: &Vec<&TxOutput>) -> f64 {
        let mut sum = 0.0;
        for tx_out in tx_outs {
            sum += tx_out.value();
        }
        sum
    }

    /// Gets the UTXOs that can be spent based on the amount to spend.
    pub fn search_utxos_to_spend(&self, amount: &f64) -> Result<Vec<&TxOutput>, NodeError> {
        let mut tx_outs_to_spend = Vec::new();

        for utxo_tuple in self.set.iter() {
            let tx_outputs = utxo_tuple.1;

            for tx_output in tx_outputs {
                tx_outs_to_spend.push(tx_output);

                if Self::sum_of_outs(&tx_outs_to_spend) >= *amount {
                    return Ok(tx_outs_to_spend);
                }
            }
        }

        Err(NodeError::NotEnoughCoins(
            "Not enough coins to spend".to_string(),
        ))
    }
}
impl Default for UtxoSet {
    /// Creates an empty UTXO set.
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_utxo_set_de_bloque_existente_contiene_txid() {
        let block_path = String::from(
            "blocks-test/0000000000000027898516270708e0c8db276e6f8302b05c8c8c208bab36ea59.bin",
        );
        let transaction_id = retrieve_transactions_from_block(&block_path).unwrap();
        let mut utxo_set = UtxoSet::new();
        utxo_set.update(&block_path).unwrap();
        assert!(utxo_set.contains_key(&transaction_id[0].tx_id()));
        assert!(utxo_set.contains_key(&transaction_id[12].tx_id()));
    }
    #[test]
    fn test_utxo_set_de_bloque_existente_contiene_tx_outputs() {
        let block_path = String::from(
            "blocks-test/0000000000000027898516270708e0c8db276e6f8302b05c8c8c208bab36ea59.bin",
        );
        let transaction_id = retrieve_transactions_from_block(&block_path).unwrap();
        let mut utxo_set = UtxoSet::new();
        utxo_set.update(&block_path).unwrap();
        let tx_outputs = utxo_set.tx_outputs(&transaction_id[12].tx_id()).unwrap();
        assert!(tx_outputs.len() == 2);
    }
    #[test]
    fn test_utxo_set_remueve_txid_cuando_esta_es_gastada() {
        let mut utxo_set = UtxoSet::new();
        let block_path1 = String::from(
            "blocks-test/0000000000000027898516270708e0c8db276e6f8302b05c8c8c208bab36ea59.bin",
        );
        utxo_set.update(&block_path1).unwrap();
        let transactions1 = retrieve_transactions_from_block(&block_path1).unwrap();
        let mut tx_outputs = utxo_set.tx_outputs(&transactions1[12].tx_id()).unwrap();
        assert!(tx_outputs.len() == 2);
        let block_path2 = String::from(
            "blocks-test/00000000000000100415543e85ed470b4c381f6adc97850c0124f367a45b4bfe.bin",
        );
        utxo_set.update(&block_path2).unwrap();

        tx_outputs = utxo_set.tx_outputs(&transactions1[12].tx_id()).unwrap();

        assert!(tx_outputs.len() == 1);
    }
}
