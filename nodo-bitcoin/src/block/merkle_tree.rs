use crate::node_error::NodeError;

use bitcoin_hashes::{sha256d, Hash};

use super::{retrieve_transactions_from_block, tx_hash::TxHash};

/// Represents a Merkle Tree, which is a binary tree where each leaf node corresponds to a transaction hash.
/// This Merkle Tree is represented as a vector of levels, where each level contains a vector of hashes.
#[derive(Debug)]
pub struct MerkleTree {
    /// A vector of levels in the Merkle tree. Each level contains a vector of hashes.
    pub leefs: Vec<Vec<TxHash>>,
}

impl MerkleTree {
    /// Creates a new, empty Merkle Tree.
    fn new() -> Self {
        MerkleTree { leefs: Vec::new() }
    }

    /// Builds a Merkle Tree from a list of transaction IDs.
    ///
    /// # Arguments
    ///
    /// * `tx_ids` - A mutable reference to a vector of transaction IDs represented as byte arrays.
    /// * `merkle_tree` - A mutable reference to the MerkleTree struct where the resulting tree will be stored.
    ///
    /// # Errors
    ///
    /// The function can return an error if there is an issue obtaining a transaction ID or if the provided Merkle tree is invalid, returns a NodeError if so.
    fn build_merkle_tree_from_hashes(
        tx_hashes: &mut Vec<TxHash>,
        merkle_tree: &mut MerkleTree,
    ) -> Result<(), NodeError> {
        if tx_hashes.len() == 1 {
            merkle_tree.push(tx_hashes);
            return Ok(());
        }

        if impar_tx_hashes(tx_hashes) {
            tx_hashes.push(
                tx_hashes
                    .last()
                    .ok_or_else(|| {
                        NodeError::InvalidMerkleRoot("Failed to obtain last txid".to_string())
                    })?
                    .to_vec(),
            );
        }

        merkle_tree.push(tx_hashes);

        let mut new_hashes = Vec::new();
        let mut i = 0;
        let tope = tx_hashes.len();
        while i < tope {
            let hash1 = tx_hashes[i].as_slice();
            let hash2 = tx_hashes[i + 1].as_slice();
            let hashes_concat = [hash1, hash2].concat();
            let hash = sha256d::Hash::hash(&hashes_concat).to_byte_array().to_vec();
            new_hashes.push(hash);
            i += 2;
        }

        Self::build_merkle_tree_from_hashes(&mut new_hashes, merkle_tree)
    }

    /// Creates a new Merkle Tree from a list of transaction hashes.
    pub fn new_from_hashes(hashes: &mut Vec<TxHash>) -> Result<Self, NodeError> {
        let mut merkle_tree = MerkleTree::new();
        Self::build_merkle_tree_from_hashes(hashes, &mut merkle_tree)?;
        Ok(merkle_tree)
    }

    /// Adds a new level of leefs to the Merkle Tree.
    pub fn push(&mut self, leef: &mut [TxHash]) {
        self.leefs.push(leef.to_vec());
    }

    /// Returns the root of the Merkle Tree, which is the Merkle Root.
    pub fn root(&self) -> &TxHash {
        &self.leefs[self.leefs.len() - 1][0]
    }

    /// Returns the number of levels in the Merkle Tree.
    pub fn levels(&self) -> usize {
        self.leefs.len()
    }
}

fn impar_tx_hashes(tx_hashes: &Vec<TxHash>) -> bool {
    tx_hashes.len() % 2 != 0
}

/// Generates a Merkle Tree from a given block.
///
/// # Arguments
///
/// * `block` - A string representing the block from which to generate the Merkle Tree.
///
/// # Errors
///
/// The function can return an error if there is an issue retrieving the transactions from the block or if there is an error in building the Merkle tree.
pub fn generate_merkle_tree(block: &String) -> Result<MerkleTree, NodeError> {
    let mut tx_ids: Vec<Vec<u8>> = retrieve_transactions_from_block(block)?
        .iter()
        .map(|tx| tx.tx_id())
        .collect();

    MerkleTree::new_from_hashes(&mut tx_ids)
}

#[cfg(test)]
mod test {

    use crate::{
        block::retrieve_transaction_ids, block_header::BlockHeader, compact_size::CompactSize,
        connectors::peer_connector::receive_message, constants::LENGTH_BLOCK_HEADERS,
    };

    use std::{
        fs::File,
        io::{Cursor, Read},
    };

    use super::*;

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
        file.read_to_end(&mut block_data).map_err(|_| {
            NodeError::FailedToRead(
                "Failed to read file block into vector in merkle tree".to_string(),
            )
        })?;
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
    fn test_merkle_tree_from_block_with_two_transactions() -> Result<(), NodeError> {
        let (mut transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        let merkle_tree = MerkleTree::new_from_hashes(&mut transaction_test_ids)?;

        let merkle_root_from_header = block_header.merkle_root_hash.to_vec();

        assert_eq!(merkle_tree.root(), &merkle_root_from_header);

        Ok(())
    }

    #[test]
    fn test_merkle_tree_from_block_with_many_transactions() -> Result<(), NodeError> {
        let (mut transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        let merkle_tree = MerkleTree::new_from_hashes(&mut transaction_test_ids)?;

        let merkle_root_from_header = block_header.merkle_root_hash.to_vec();

        assert_eq!(merkle_tree.root(), &merkle_root_from_header);

        Ok(())
    }
}
