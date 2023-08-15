use glib::Sender;

use crate::{
    block_header::{block_header_bytes::BlockHeaderBytes, BlockHeader},
    compact_size::CompactSize,
    config::obtain_dir_path,
    connectors::peer_connector::receive_message,
    constants::{BLOCK_HEADERS_FILE, LENGTH_BLOCK_HEADERS},
    node_error::NodeError,
    transactions::transaction::Transaction,
    ui::ui_message::UIMessage,
};

use std::{
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Write},
};

use self::{merkle_tree::MerkleTree, tx_hash::TxHash};

pub mod block_hash;
pub mod hash_direction_tuple;
pub mod merkle_tree;
pub mod proof_of_inclusion;
pub mod tx_hash;

/// Validates a block's Merkle Root.
///
/// # Arguments
///
/// * `block_header` - A reference to the block header containing the valid Merkle Root.
/// * `block_txs` - A mutable reference to a vector containing the transaction data of the block.
///
/// # Returns
///
/// A `Result` indicating the result of the validation. If the Merkle Root is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
pub fn validate_merkle_root(
    block_header: &BlockHeader,
    block_txs: &mut Vec<TxHash>,
) -> Result<(), NodeError> {
    let valid_merkle_root = &block_header.merkle_root_hash.to_vec();

    let merkle_tree = MerkleTree::new_from_hashes(block_txs)?;

    if valid_merkle_root != merkle_tree.root() {
        return Err(NodeError::InvalidMerkleRoot(
            "Invalid merkle root".to_string(),
        ));
    }

    Ok(())
}

/// Validates the proof-of-work of a block.
///
/// # Arguments
///
/// * `block_header` - A reference to the block header to validate.
///
/// # Returns
///
/// A `Result` indicating the result of the validation. If the proof-of-work is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
pub fn validate_proof_of_work(block_header: &BlockHeader) -> Result<(), NodeError> {
    let target_threshold = block_header.calculate_target_threshold().to_vec();
    let mut hash = block_header.hash().clone();
    hash.reverse();

    if hash > target_threshold {
        return Err(NodeError::InvalidProofOfWork(
            "Invalid target threshold".to_string(),
        ));
    }

    Ok(())
}
/// Validates a block.
///
/// # Arguments
///
/// * `block_header` - A reference to the block header to validate.
/// * `block_txs` - A mutable reference to a vector containing the transaction data of the block.
///
/// # Returns
///
/// A `Result` indicating the result of the validation. If the block is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
pub fn validate_block(
    block_header: &BlockHeader,
    block_txs: &mut Vec<TxHash>,
) -> Result<(), NodeError> {
    validate_proof_of_work(block_header)?;
    validate_merkle_root(block_header, block_txs)
}

/// Retrieves transaction IDs from a TCP stream.
///
/// # Arguments
///
/// * `source` - A mutable reference to a something that implements te Read trait, might be a file, stream, etc.
/// * `txs_count` - The number of transactions to retrieve.
///
/// # Returns
///
/// A `Result` containing a vector of transaction IDs.
pub fn retrieve_transaction_ids<R: Read>(
    source: &mut R,
    txs_count: u64,
) -> Result<Vec<TxHash>, NodeError> {
    let mut transaction_ids = Vec::new();

    let mut transaction = Transaction::read_coinbase_transaction(source)?;
    let mut tx_id = transaction.tx_id();
    transaction_ids.push(tx_id);

    for _ in 1..txs_count {
        transaction = Transaction::read_transaction(source)?;
        tx_id = transaction.tx_id();
        transaction_ids.push(tx_id);
    }
    Ok(transaction_ids)
}

/// Retrieves transactions from an object that implements the read trait.
///
/// Given the number of transactions to read (`txs_count_value`) and a file
/// handle (`file`), this function reads the specified number of transactions
/// from the file and returns them as a vector of `Transaction` objects.
///
/// # Arguments
///
/// * `txs_count_value` - The number of transactions to read from the file.
/// * `file` - A `File` handle representing the open file to read from.
///
/// # Returns
///
/// A `Result` containing a vector of `Transaction` objects on success, or an
/// error of type `NodeError` if there was an issue reading the transactions
/// from the file.
pub fn retrieve_transactions<R: Read>(
    source: &mut R,
    txs_count_value: u64,
) -> Result<Vec<Transaction>, NodeError> {
    let mut transactions = Vec::new();
    for _ in 0..txs_count_value {
        let transaction = Transaction::read_transaction(source)?;
        transactions.push(transaction);
    }
    Ok(transactions)
}

/// Writes a block to disk.
/// # Arguments
/// * `block_data` - A vector of bytes containing the block data.
/// * `path` - A reference to a string containing the path to the blocks directory.
/// # Returns
/// A `Result` indicating the result of the writing. If the block is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
fn write_block_to_disk(block_data: Vec<u8>, path: &String) -> Result<(), NodeError> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| NodeError::FailedToOpenFile("Failed to open file".to_string()))?;

    file.write_all(&block_data)
        .map_err(|_| NodeError::FailedToWrite("Failed to write block to file".to_string()))?;
    Ok(())
}

/// Write a block header to a file.
///
/// This function takes a reference to a `BlockHeaderBytes` and writes it to a file named
/// `BLOCK_HEADERS_FILE`. The file is created if it does not exist, and the block header is
/// appended to the end of the file. If the file already exists, the block header is written
/// after the current contents.
///
/// # Arguments
///
/// * `block_header` - A reference to a `BlockHeaderBytes` representing the block header data
///                    that needs to be written to the file.
///
/// # Errors
///
/// This function can return a `NodeError` in case of any of the following errors:
///
/// * `FailedToOpenFile` - If the function fails to open the file for writing or creating.
/// * `FailedToWriteAll` - If the function fails to write the block header to the file.
pub fn write_block_header_to_file(block_header: &BlockHeaderBytes) -> Result<(), NodeError> {
    let dir_headers_file = obtain_dir_path(BLOCK_HEADERS_FILE.to_owned())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(dir_headers_file)
        .map_err(|_| {
            NodeError::FailedToOpenFile("Failed to open block headers file".to_string())
        })?;

    file.write_all(block_header).map_err(|_| {
        NodeError::FailedToWriteAll("Failed to write block header to file".to_string())
    })?;

    Ok(())
}

/// Handles a new block message.
///
/// # Arguments
/// * `block_data` - A vector of bytes containing the block data.
/// * `path` - A reference to a string containing the path to the blocks directory.
///
/// # Returns
///
/// A `Result` indicating the result of the handling. If the block is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
pub fn validate_and_save_block(block_data: Vec<u8>, path: &String) -> Result<(), NodeError> {
    let mut cursor = Cursor::new(&block_data);
    let block_header_bytes = receive_message(&mut cursor, LENGTH_BLOCK_HEADERS)?;
    let block_header = BlockHeader::from_bytes(&block_header_bytes)?;
    let txs_count = CompactSize::read_varint(&mut cursor)?;
    let txs_count_value = txs_count.get_value();
    let mut transaction_ids = retrieve_transaction_ids(&mut cursor, txs_count_value)?;

    match validate_block(&block_header, &mut transaction_ids) {
        Ok(()) => {
            write_block_to_disk(block_data, path)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Handles a new block message.
///
/// # Arguments
/// * `block_data` - A vector of bytes containing the block data.
/// * `path` - A reference to a string containing the path to the blocks directory.
///
/// # Returns
///
/// A `Result` indicating the result of the handling. If the block is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
pub fn validate_and_save_block_listener(
    block_data: Vec<u8>,
    path: &String,
    ui_sender: &Sender<UIMessage>,
) -> Result<(), NodeError> {
    let mut cursor = Cursor::new(&block_data);
    let block_header_bytes = receive_message(&mut cursor, LENGTH_BLOCK_HEADERS)?;
    let block_header = BlockHeader::from_bytes(&block_header_bytes)?;
    let txs_count = CompactSize::read_varint(&mut cursor)?;
    let txs_count_value = txs_count.get_value();
    let mut transaction_ids = retrieve_transaction_ids(&mut cursor, txs_count_value)?;

    match validate_block(&block_header, &mut transaction_ids) {
        Ok(()) => {
            ui_sender
                .send(UIMessage::NewBlock(block_header))
                .unwrap_or_else(|_| {
                    println!("Failed to send new block message to UI thread");
                });
            write_block_to_disk(block_data, path)?;
            write_block_header_to_file(&block_header_bytes)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Retrieves the transactions from a block file.
///
/// # Arguments
///
/// * `path` - A string reference representing the file path to the block file.
///
/// # Returns
///
/// A `Result` containing a vector of `Transaction` objects on success, or an
/// error of type `NodeError` if there was an issue opening or reading the file.
pub fn retrieve_transactions_from_block(path: &String) -> Result<Vec<Transaction>, NodeError> {
    let mut file = File::options()
        .read(true)
        .open(path)
        .map_err(|_| NodeError::FailedToOpenFile("Failed to open file block".to_string()))?;

    let block_header_bytes = receive_message(&mut file, LENGTH_BLOCK_HEADERS)?;
    let _block_header = BlockHeader::from_bytes(&block_header_bytes)?;
    let txs_count = CompactSize::read_varint(&mut file)?;
    let txs_count_value = txs_count.get_value();

    let transactions = retrieve_transactions(&mut file, txs_count_value)?;

    Ok(transactions)
}

#[cfg(test)]
mod test {

    use std::{
        env,
        io::{BufRead, BufReader},
    };

    use crate::{config::parse_line, constants::DEFAULT_CONFIG};
    use bitcoin_hashes::hex::FromHex;

    use super::*;

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

    /// Retrieves the transaction IDs and block header from a block file.
    ///
    /// # Arguments
    ///
    /// * `path` - A string representing the file path to the block file.
    ///
    /// # Returns
    ///
    /// A tuple containing the vector of transaction IDs and the block header,
    /// wrapped in a `Result`. If the block file is valid, the `Result` will be `Ok`, if its not it will return a `NodeError`.
    fn get_transactions_id_from_block(
        path: String,
    ) -> Result<(Vec<Vec<u8>>, BlockHeader), NodeError> {
        let mut file = File::options()
            .read(true)
            .open(path)
            .map_err(|_| NodeError::FailedToOpenFile("Failed to open file block".to_string()))?;

        // Obtener el tamaño del archivo
        let file_size = file
            .metadata()
            .map_err(|_| NodeError::FailedToOpenFile("Failed to obtain file len".to_string()))?
            .len() as usize;

        // Crear un vector con capacidad suficiente para almacenar el contenido del archivo
        let mut block_data = Vec::with_capacity(file_size);

        // Leer el contenido del archivo en el vector
        file.read_to_end(&mut block_data).unwrap();
        //let data_to_write = block_data.clone();
        let mut cursor = Cursor::new(&block_data);

        let block_header_bytes = receive_message(&mut cursor, LENGTH_BLOCK_HEADERS)?;
        let block_header = BlockHeader::from_bytes(&block_header_bytes)?;
        let txs_count = CompactSize::read_varint(&mut cursor)?;
        let txs_count_value = txs_count.get_value();

        let transaction_ids = retrieve_transaction_ids(&mut cursor, txs_count_value)?;
        Ok((transaction_ids, block_header))
    }

    #[test]
    fn test_handle_new_block_validation() -> Result<(), NodeError> {
        load_default_config()?;
        let mut file = File::options()
            .read(true)
            .open(
                "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin",
            )
            .map_err(|_| NodeError::FailedToOpenFile("Failed to open file block".to_string()))?;

        let mut block_data = Vec::new();
        file.read_to_end(&mut block_data)
            .map_err(|_| NodeError::FailedToRead("Failed to read file".to_string()))?;

        let mut cursor = Cursor::new(&block_data);
        let block_header_bytes = receive_message(&mut cursor, LENGTH_BLOCK_HEADERS)?;
        let block_header = BlockHeader::from_bytes(&block_header_bytes)?;
        let txs_count = CompactSize::read_varint(&mut cursor)?;
        let txs_count_value = txs_count.get_value();

        let mut transaction_ids = retrieve_transaction_ids(&mut cursor, txs_count_value)?;

        let mut new_file = File::options()
            .create(true)
            .write(true)
            .open("blocks-test/test_validation.bin")
            .map_err(|_| NodeError::FailedToOpenFile("Failed to open file block".to_string()))?;

        match validate_block(&block_header, &mut transaction_ids) {
            Ok(()) => {
                new_file
                    .write_all(&block_data)
                    .map_err(|_| NodeError::FailedToWrite("Failed to write to file".to_string()))?;
                return Ok(());
            }
            Err(e) => return Err(e),
        }
    }
    #[test]
    fn test_proof_of_work1() -> Result<(), NodeError> {
        let (_transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        assert!(validate_proof_of_work(&block_header).is_ok());

        Ok(())
    }

    #[test]
    fn test_proof_of_work2() -> Result<(), NodeError> {
        let (_transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        assert!(validate_proof_of_work(&block_header).is_ok());

        Ok(())
    }

    #[test]
    fn test_proof_of_work3() -> Result<(), NodeError> {
        let (_transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000000a2b6d192ab83f7706e60cece100aabb45a4b9ce4656b6a702.bin"
                .to_string(),
        )?;

        assert!(validate_proof_of_work(&block_header).is_ok());

        Ok(())
    }

    #[test]
    fn test_correct_transaction_hashes() -> Result<(), NodeError> {
        let (transaction_test_hashes, _block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;
        let mut coinbase =
            Vec::from_hex("7e92b6982a89b2bcb7f40c9cbd05db22d46bb08b1b0a001a40e4fca0b49f80a9")
                .unwrap();
        coinbase.reverse();

        let mut tx_7 =
            Vec::from_hex("5b32f673a000733900a2208388a6da5e2d21306b935b6bfdaca3982e4315db09")
                .unwrap();
        tx_7.reverse();

        let mut tx_50 =
            Vec::from_hex("38b4025adf72314d8d80536c3a7fe42e86a301e5a22b2d156cc946b81ce8b9a8")
                .unwrap();
        tx_50.reverse();

        assert_eq!(transaction_test_hashes[0], coinbase);
        assert_eq!(transaction_test_hashes[7], tx_7);
        assert_eq!(transaction_test_hashes[50], tx_50);

        Ok(())
    }

    #[test]
    fn test_block_with_two_transactions() -> Result<(), NodeError> {
        let (mut transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        let merkle_tree = MerkleTree::new_from_hashes(&mut transaction_test_ids)?;

        let merkle_root_from_header = &block_header.merkle_root_hash.to_vec();

        assert_eq!(merkle_tree.root(), merkle_root_from_header);

        Ok(())
    }

    #[test]
    fn test_block_with_many_transactions() -> Result<(), NodeError> {
        let (mut transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        let merkle_tree = MerkleTree::new_from_hashes(&mut transaction_test_ids)?;

        let merkle_root_from_header = &block_header.merkle_root_hash.to_vec();

        assert_eq!(merkle_tree.root(), merkle_root_from_header);

        Ok(())
    }

    #[test]
    fn test_validate_block1() -> Result<(), NodeError> {
        let (mut transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        assert!(validate_block(&block_header, &mut transaction_test_hashes).is_ok());

        Ok(())
    }

    #[test]
    fn test_validate_block2() -> Result<(), NodeError> {
        let (mut transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/0000000064cb619ed029a4dec104c841bc97127054918a3275237900b5944cff.bin"
                .to_string(),
        )?;

        assert!(validate_block(&block_header, &mut transaction_test_hashes).is_ok());

        Ok(())
    }

    #[test]
    fn test_validate_block3() -> Result<(), NodeError> {
        let (mut transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000000c76b5b8f47558c66d03cd0a898df32b33f9e63a39abc5be5f.bin"
                .to_string(),
        )?;

        assert!(validate_block(&block_header, &mut transaction_test_hashes).is_ok());

        Ok(())
    }

    #[test]
    fn test_validate_block4() -> Result<(), NodeError> {
        load_default_config()?;
        let (mut transaction_test_hashes, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        assert!(validate_block(&block_header, &mut transaction_test_hashes).is_ok());

        Ok(())
    }
}
