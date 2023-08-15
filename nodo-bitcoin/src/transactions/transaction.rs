use std::io::Read;

use bitcoin_hashes::{sha256, sha256d, Hash};

use crate::{
    block::tx_hash::TxHash,
    compact_size::CompactSize,
    connectors::peer_connector::receive_message,
    constants::{LENGTH_LOCK_TIME, LENGTH_VERSION, SIGHASH_ALL},
    node_error::NodeError,
    ui::components::transactions_confirmed_data::Amount,
    wallet::{account::Account, bitcoin_address::BitcoinAddress},
};

use super::{
    pk_script::PkScript, signature_script::SignatureScript, tx_input::TxInput, tx_output::TxOutput,
};

#[derive(Debug, Clone)]
/// Represents a blocks transaction.
pub struct Transaction {
    /// The version of the transaction.
    pub version: u32,
    /// The number of transaction inputs.
    pub tx_in_count: CompactSize,
    /// The transaction inputs.
    pub tx_inputs: Vec<TxInput>,
    /// The number of transaction outputs.
    pub tx_out_count: CompactSize,
    /// The transaction outputs.
    pub tx_outputs: Vec<TxOutput>,
    /// The lock time of the transaction.
    pub lock_time: u32,
}

impl Transaction {
    /// Reads a transaction from a reader.
    ///
    /// # Arguments
    ///
    /// * `block` - A mutable reference to a reader implementing the `Read` trait, can be a file, TcpStream, etc.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `Transaction` if successful, or a `NodeError` if an error occurs.
    pub fn read_transaction<R: Read>(block: &mut R) -> Result<Transaction, NodeError> {
        let version = receive_message(block, LENGTH_VERSION)?;

        //Input
        let tx_in_count = CompactSize::read_varint(block)?;
        let mut tx_inputs = Vec::new();
        let tx_in_count_value = tx_in_count.get_value();

        for _ in 0..tx_in_count_value {
            let tx_input = TxInput::read_tx_input(block)?;
            tx_inputs.push(tx_input);
        }

        //Output
        let tx_out_count = CompactSize::read_varint(block)?;
        let mut tx_outputs = Vec::new();
        let tx_out_count_value = tx_out_count.get_value();

        for i in 0..tx_out_count_value {
            let tx_output = TxOutput::read_tx_output_from_block(block, i)?;
            tx_outputs.push(tx_output);
        }

        //Lock time
        let lock_time = receive_message(block, LENGTH_LOCK_TIME)?;

        let mut tx = Transaction {
            version: u32::from_le_bytes([version[0], version[1], version[2], version[3]]),
            tx_in_count,
            tx_inputs,
            tx_out_count,
            tx_outputs,
            lock_time: u32::from_le_bytes([lock_time[0], lock_time[1], lock_time[2], lock_time[3]]),
        };

        tx.add_tx_id_to_tx_outs();
        Ok(tx)
    }

    /// Retrieves the coinbase transaction from a block, this is the first transaction in a block.
    ///
    /// # Arguments
    ///
    /// * `block` - A mutable reference to a reader implementing the `Read` trait, such as a file or a `TcpStream`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the parsed `Transaction` if successful, or a `NodeError` if an error occurs.
    pub fn read_coinbase_transaction<R: Read>(block: &mut R) -> Result<Transaction, NodeError> {
        let version = receive_message(block, LENGTH_VERSION)?;

        //Input
        let tx_in_count = CompactSize::read_varint(block)?;
        let _tx_in_count_value = tx_in_count.get_value();
        let tx_inputs = vec![TxInput::read_tx_coinbase_input(block)?];

        //Output
        let tx_out_count = CompactSize::read_varint(block)?;
        let mut tx_outputs = Vec::new();
        let tx_out_count_value = tx_out_count.get_value();

        for i in 0..tx_out_count_value {
            let tx_output = TxOutput::read_tx_output_from_block(block, i)?;

            tx_outputs.push(tx_output);
        }

        //Lock time
        let lock_time = receive_message(block, LENGTH_LOCK_TIME)?;

        let mut tx = Transaction {
            version: u32::from_le_bytes([version[0], version[1], version[2], version[3]]),
            tx_in_count,
            tx_inputs,
            tx_out_count,
            tx_outputs,
            lock_time: u32::from_le_bytes([lock_time[0], lock_time[1], lock_time[2], lock_time[3]]),
        };

        tx.add_tx_id_to_tx_outs();

        Ok(tx)
    }

    /// Creates a new transaction with unsigned inputs.
    pub fn new_unsigned(unsigned_tx_ins: Vec<TxInput>, tx_outs: Vec<TxOutput>) -> Transaction {
        Transaction {
            version: 1,
            tx_in_count: CompactSize::new(unsigned_tx_ins.len()),
            tx_inputs: unsigned_tx_ins,
            tx_out_count: CompactSize::new(tx_outs.len()),
            tx_outputs: tx_outs,
            lock_time: 0,
        }
    }

    /// Converts the transaction to a byte representation.
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the transaction.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.version.to_le_bytes().to_vec());
        bytes.extend(self.tx_in_count.to_bytes());

        for tx_input in &self.tx_inputs {
            bytes.extend(tx_input.to_bytes());
        }

        bytes.extend(self.tx_out_count.to_bytes());
        for tx_output in &self.tx_outputs {
            bytes.extend(tx_output.to_bytes());
        }
        bytes.extend(self.lock_time.to_le_bytes().to_vec());
        bytes
    }

    /// Gets the transaction id.
    pub fn tx_id(&self) -> TxHash {
        let tx_bytes = self.to_bytes();
        sha256d::Hash::hash(&tx_bytes).to_byte_array().to_vec()
    }

    /// Generates an individual signature for a specific input.
    ///
    /// # Arguments
    ///
    /// * `i` - The index of the transaction input for which the signature is being generated.
    /// * `pk_script` - The PkScript associated with the input being signed.
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the signature.
    pub fn individual_signature_hash(&mut self, i: usize, pk_script: PkScript) -> Vec<u8> {
        self.tx_inputs[i].script_bytes = CompactSize::new(pk_script.len());
        self.tx_inputs[i].signature_script = pk_script;

        let mut tx_bytes = self.to_bytes();
        tx_bytes.append(&mut SIGHASH_ALL.to_le_bytes().to_vec());

        let sig_hash = sha256::Hash::hash(&tx_bytes).to_byte_array().to_vec();

        self.tx_inputs[i].script_bytes = CompactSize::new(0);
        self.tx_inputs[i].signature_script = vec![];

        sig_hash
    }

    /// Adds the script signatures to the transaction inputs.
    pub fn add_script_sigs(&mut self, script_sigs: Vec<SignatureScript>) {
        for (i, script_sig) in script_sigs.iter().enumerate().take(self.tx_inputs.len()) {
            self.tx_inputs[i].script_bytes = CompactSize::new(script_sig.len());
            self.tx_inputs[i].signature_script = script_sig.clone();
        }
    }

    /// Adds the transaction id to the transaction outputs.
    pub fn add_tx_id_to_tx_outs(&mut self) {
        let tx_id = self.tx_id();
        for tx_output in &mut self.tx_outputs {
            tx_output.tx_id = tx_id.clone();
        }
    }

    /// Adds the block path to the transaction outputs.
    pub fn add_block_path_to_tx_outs(&mut self, block_path: &str) {
        for tx_output in &mut self.tx_outputs {
            tx_output.block_path = block_path.to_owned();
        }
    }

    /// Gets a reference to the transaction outputs.
    pub fn tx_outputs(&self) -> &Vec<TxOutput> {
        &self.tx_outputs
    }

    /// Gets a reference to the transaction inputs.
    pub fn tx_inputs(&self) -> &Vec<TxInput> {
        &self.tx_inputs
    }

    /// Checks if the Transaction contains a specific bitcoin address.
    pub fn contains_address(&self, address: &BitcoinAddress) -> bool {
        for tx_output in &self.tx_outputs {
            if tx_output.contains_address(address) {
                println!("Transaction contains address: {:?}", address);
                return true;
            }
        }
        false
    }

    /// Gets the amount of bitcoin received by a specific address.
    /// # Arguments
    /// * `address` - The address to check.
    /// # Returns
    /// The amount of bitcoin received by the address.
    pub fn amount_received_by_address(&self, address: &BitcoinAddress) -> Amount {
        let mut amount: f64 = 0.0;
        for tx_output in &self.tx_outputs {
            if tx_output.contains_address(address) {
                amount += tx_output.value();
            }
        }
        amount.to_string()
    }

    /// Gets the amount of bitcoin spent by a specific address.
    /// # Arguments
    /// * `account` - The account to check.
    /// # Returns
    /// The amount of bitcoin spent by the account.
    pub fn amount_spent_by_account(&self, account: &mut Account) -> f64 {
        let bitcoin_address = &account.bitcoin_address();
        let mut utxo_set = account.utxo_set.clone();
        let mut input = 0.0;
        for tx_input in &self.tx_inputs {
            if let Some(tx_outputs) = utxo_set.tx_outputs(&tx_input.previous_output.tx_id) {
                for tx in tx_outputs {
                    if tx.contains_address(bitcoin_address) {
                        input += tx.value();
                    }
                }
            }
        }
        input
    }
}
