use bitcoin_hashes::{sha256d, Hash};

use crate::{
    constants::{LEFT, RIGHT},
    node_error::NodeError,
    utils::Utils,
};

use super::{
    hash_direction_tuple::HashDirectionTuple,
    merkle_tree::{generate_merkle_tree, MerkleTree},
    tx_hash::TxHash,
};

/// Represents a Merkle proof for a specific transaction in a Merkle tree.
#[derive(Debug)]
pub struct MerkleProof {
    /// The proof path consisting of tuples containing the transaction hash (tx id) and direction.
    pub proof_path: Vec<HashDirectionTuple>,
}

impl MerkleProof {
    /// Determines the direction of a leaf node in the Merkle Tree based on the given transaction ID.
    ///
    /// # Arguments
    ///
    /// * `tx_id` - A reference to the transaction ID as a vector of bytes.
    /// * `merkle_tree` - A reference to the Merkle Tree.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The direction of the leaf node ("left" or "right") if the transaction ID is found in the Merkle tree.
    /// * `Err(NodeError)` - An error indicating failure to obtain the hash index or transaction not found in the block.
    fn leaf_node_direction(tx_id: &TxHash, merkle_tree: &MerkleTree) -> Result<String, NodeError> {
        let hash_index =
            match merkle_tree.leefs[0].iter().position(|h| h == tx_id) {
                Some(index) => index,
                None => return Err(NodeError::InvalidMerkleTree(
                    "Failed to obtain hash index from merkle tree, transaction not found in block"
                        .to_string(),
                )),
            };

        Ok(if hash_index % 2 == 0 { LEFT } else { RIGHT }.to_string())
    }

    /// Initializes a Merkle Proof by constructing the proof path starting from a leaf node identified by the given transaction ID.
    ///
    /// The function takes ownership of a vector representing the transaction ID (`tx_id`) and a reference to a `MerkleTree` (`merkle_tree`).
    /// It determines the direction of the leaf node in the Merkle tree based on the transaction ID.
    ///
    /// # Arguments
    ///
    /// * `tx_id` - A vector representing the transaction ID.
    /// * `merkle_tree` - A reference to the Merkle tree.
    ///
    /// # Returns
    ///
    /// * `Ok(MerkleProof)` - The initialized `MerkleProof` struct containing the proof path.
    /// * `Err(NodeError)` - An error indicating failure to determine the leaf node direction or transaction not found in the block.
    fn initialize_from_leaf(
        mut tx_id: TxHash,
        merkle_tree: &MerkleTree,
    ) -> Result<MerkleProof, NodeError> {
        let mut proof_path = Vec::new();
        let leaf_node_direction = Self::leaf_node_direction(&tx_id, merkle_tree)?;
        tx_id.reverse();
        proof_path.push((tx_id, leaf_node_direction));

        Ok(MerkleProof { proof_path })
    }

    /// Pushes a new hash and direction to the proof path.
    fn push(&mut self, hash: TxHash, direction: String) {
        self.proof_path.push((hash, direction));
    }

    /// Generates a Merkle Proof for a transaction within a given block.
    ///
    /// It generates a Merkle tree from the block and initializes a `MerkleProof` struct starting from the given transaction ID.
    /// It then iteratively adds sibling hashes and their directions to the proof path until reaching the root of the Merkle tree.
    /// The function returns the generated `MerkleProof`.
    ///
    /// # Arguments
    ///
    /// * `tx_id` - The transaction ID as a string.
    /// * `block` - The block path as a string.
    ///
    /// # Returns
    ///
    /// * `Ok(MerkleProof)` - The generated `MerkleProof` containing the proof path.
    /// * `Err(NodeError)` - An error indicating failure to generate the Merkle Proof.
    pub fn for_tx_in_block(tx_id: String, block: String) -> Result<MerkleProof, NodeError> {
        let merkle_tree = generate_merkle_tree(&block)?;
        let mut tx_id_bytes = Utils::hex_string_to_bytes(tx_id)?;
        tx_id_bytes.reverse();

        let mut merkle_proof =
            MerkleProof::initialize_from_leaf(tx_id_bytes.clone(), &merkle_tree)?;

        let mut hash_index =
            match merkle_tree.leefs[0].iter().position(|h| h == &tx_id_bytes) {
                Some(index) => index,
                None => return Err(NodeError::InvalidMerkleTree(
                    "Failed to obtain hash index from merkle tree, transaction not found in block"
                        .to_string(),
                )),
            };

        for level in 0..merkle_tree.levels() - 1 {
            let is_left_child = hash_index % 2 == 0;

            let sibling_direction = if is_left_child { RIGHT } else { LEFT }.to_string();

            let sibling_index = if is_left_child {
                hash_index + 1
            } else {
                hash_index - 1
            };

            let mut sib = merkle_tree.leefs[level][sibling_index].clone();
            sib.reverse();

            merkle_proof.push(sib, sibling_direction);
            hash_index /= 2;
        }

        Ok(merkle_proof)
    }

    /// Builds and returns the Merkle Root hash based on the Merkle Proof path. It is used over a MerkleProof object that has been initialized.
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - The computed Merkle root hash as a byte array.
    pub fn build_merkle_root(&self) -> TxHash {
        let mut merkle_root = self.proof_path[0].0.clone();
        merkle_root.reverse();

        for (sibling_hash, sibling_direction) in &self.proof_path[1..] {
            let mut sibling_hash = sibling_hash.clone();
            sibling_hash.reverse();

            if sibling_direction == RIGHT {
                merkle_root = [merkle_root, sibling_hash].concat();
            } else {
                merkle_root = [sibling_hash, merkle_root].concat();
            }

            merkle_root = sha256d::Hash::hash(&merkle_root).to_byte_array().to_vec();
        }

        merkle_root
    }

    /// Determines whether a transaction is included in a given block.
    pub fn determine_inclusion_for_tx_in_block(
        tx_id: String,
        block: String,
    ) -> Result<bool, NodeError> {
        let merkle_tree = generate_merkle_tree(&block)?;
        let proof_of_inclusion = MerkleProof::for_tx_in_block(tx_id, block)?;

        let trees_merkle_root = merkle_tree.root();
        let inclusions_merkle_root = &proof_of_inclusion.build_merkle_root();

        Ok(trees_merkle_root == inclusions_merkle_root)
    }

    /// Returns the string representation of the Merkle Proof path.
    pub fn to_string_format(&self) -> String {
        let mut path = String::new();

        path.push_str(&format!(
            "Merkle Proof for {} =\n",
            Utils::bytes_to_hex(&self.proof_path[0].0)
        ));

        for hash_direction_tuple in &self.proof_path {
            let hash = Utils::bytes_to_hex(&hash_direction_tuple.0);
            let direction = &hash_direction_tuple.1;

            path.push_str("{\n");
            path.push_str(&format!("    hash: '{}',\n", hash));
            path.push_str(&format!("    direction: '{}'\n", direction));
            path.push_str("},\n");
        }

        path
    }

    /// Returns the string representation of the Merkle Proof path for a given transaction in a given block.
    pub fn path_for_tx_in_block(tx_id: String, block: String) -> Result<String, NodeError> {
        let merkle_proof = MerkleProof::for_tx_in_block(tx_id, block)?;

        Ok(merkle_proof.to_string_format())
    }
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

        let file_size = file
            .metadata()
            .map_err(|_| NodeError::FailedToOpenFile("Failed to obtain file len".to_string()))?
            .len() as usize;

        let mut block_data = Vec::with_capacity(file_size);

        file.read_to_end(&mut block_data).map_err(|_| {
            NodeError::FailedToRead(
                "Failed to read file block to end in proof of inclusion".to_string(),
            )
        })?;

        let mut cursor = Cursor::new(&block_data);

        let block_header_bytes = receive_message(&mut cursor, LENGTH_BLOCK_HEADERS)?;
        let block_header = BlockHeader::from_bytes(&block_header_bytes)?;
        let txs_count = CompactSize::read_varint(&mut cursor)?;
        let txs_count_value = txs_count.get_value();

        let transaction_ids = retrieve_transaction_ids(&mut cursor, txs_count_value)?;
        Ok((transaction_ids, block_header))
    }

    #[test]
    fn test_merkle_proof_from_block_with_two_transactions() -> Result<(), NodeError> {
        let (_transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        let merkle_proof = MerkleProof::for_tx_in_block(
            "9b7314b2ba807c45c7dd7683b0e966a1b97ab00fc476d60fd8caf88e614bcda5".to_string(),
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        assert_eq!(
            merkle_proof.build_merkle_root(),
            block_header.merkle_root_hash.to_vec()
        );

        println!("{}", merkle_proof.to_string_format());

        Ok(())
    }

    #[test]
    fn test_merkle_proof_from_block_with_two_transactions_but_with_otherone(
    ) -> Result<(), NodeError> {
        let (_transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        let merkle_proof = MerkleProof::for_tx_in_block(
            "3784b1bc98c477e27f7b035091b4b0f08abaab916acb949a62fd4a4ad7ae621c".to_string(),
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string(),
        )?;

        assert_eq!(
            merkle_proof.build_merkle_root(),
            block_header.merkle_root_hash.to_vec()
        );

        println!("{}", merkle_proof.to_string_format());

        Ok(())
    }

    #[test]
    fn test_merkle_proof_from_block_with_two_transactions_but_with_unexisting_tx(
    ) -> Result<(), NodeError> {
        assert!(MerkleProof::for_tx_in_block(
            "3784b1bc98c477e27f7b035091b4b0f08abaab916acb949a62fd4a4ad7ae622c".to_string(),
            "blocks-test/000000000000001035138c7d63a9f79a25afc119403e2384d8ad285bce01bf8b.bin"
                .to_string()
        )
        .is_err());

        Ok(())
    }

    #[test]
    fn test_merkle_proof_from_block_with_many_transactions() -> Result<(), NodeError> {
        let (_transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        let merkle_proof = MerkleProof::for_tx_in_block(
            "5b32f673a000733900a2208388a6da5e2d21306b935b6bfdaca3982e4315db09".to_string(),
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        assert_eq!(
            Utils::bytes_to_hex(&merkle_proof.proof_path[0].0),
            "5b32f673a000733900a2208388a6da5e2d21306b935b6bfdaca3982e4315db09".to_string()
        );

        let mut merkle_root_bloque = block_header.merkle_root_hash.to_vec();
        merkle_root_bloque.reverse();

        let mut merkle_root_de_proof = merkle_proof.build_merkle_root();
        merkle_root_de_proof.reverse();

        // si se busca la merkle root en un explorador se puede ver que es 55a61eb710b66ed7f6f6c8a1b20451f971f48d8dc7a6326b4670601ba454c29e, que es la misma.
        println!(
            "Proof's Merkle Root: {:?}",
            Utils::bytes_to_hex(&merkle_root_de_proof)
        );

        assert_eq!(merkle_root_de_proof, merkle_root_bloque);
        assert!(MerkleProof::determine_inclusion_for_tx_in_block(
            "5b32f673a000733900a2208388a6da5e2d21306b935b6bfdaca3982e4315db09".to_string(),
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string()
        )?);

        println!("{}", merkle_proof.to_string_format());

        Ok(())
    }

    #[test]
    fn test_merkle_proof_from_block_with_many_transactions_pero_otra_tx_random(
    ) -> Result<(), NodeError> {
        let (_transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        let merkle_proof = MerkleProof::for_tx_in_block(
            "530d7c4e56c5133c05fd0b0b56e9245c27897cceb35fa3affd60fe47539a72bc".to_string(),
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        assert_eq!(
            Utils::bytes_to_hex(&merkle_proof.proof_path[0].0),
            "530d7c4e56c5133c05fd0b0b56e9245c27897cceb35fa3affd60fe47539a72bc".to_string()
        );

        let mut merkle_root_bloque = block_header.merkle_root_hash.to_vec();
        merkle_root_bloque.reverse();

        let mut merkle_root_de_proof = merkle_proof.build_merkle_root();
        merkle_root_de_proof.reverse();

        println!(
            "Proof's Merkle Root: {:?}",
            Utils::bytes_to_hex(&merkle_root_de_proof)
        );

        assert_eq!(merkle_root_de_proof, merkle_root_bloque);
        assert!(MerkleProof::determine_inclusion_for_tx_in_block(
            "530d7c4e56c5133c05fd0b0b56e9245c27897cceb35fa3affd60fe47539a72bc".to_string(),
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string()
        )?);

        println!("{}", merkle_proof.to_string_format());

        Ok(())
    }

    #[test]
    fn test_merkle_proof_from_block_with_many_transactions_pero_otra_tx_random_mas(
    ) -> Result<(), NodeError> {
        let (_transaction_test_ids, block_header) = get_transactions_id_from_block(
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        let merkle_proof = MerkleProof::for_tx_in_block(
            "3e1cfe72a326ff21b00ba424cb396a48ce75e44bbafc21fabd549e4cbf884b34".to_string(),
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string(),
        )?;

        assert_eq!(
            Utils::bytes_to_hex(&merkle_proof.proof_path[0].0),
            "3e1cfe72a326ff21b00ba424cb396a48ce75e44bbafc21fabd549e4cbf884b34".to_string()
        );

        let mut merkle_root_bloque = block_header.merkle_root_hash.to_vec();
        merkle_root_bloque.reverse();

        let mut merkle_root_de_proof = merkle_proof.build_merkle_root();
        merkle_root_de_proof.reverse();

        println!(
            "Proof's Merkle Root: {:?}",
            Utils::bytes_to_hex(&merkle_root_de_proof)
        );

        assert_eq!(merkle_root_de_proof, merkle_root_bloque);
        assert!(MerkleProof::determine_inclusion_for_tx_in_block(
            "3e1cfe72a326ff21b00ba424cb396a48ce75e44bbafc21fabd549e4cbf884b34".to_string(),
            "blocks-test/00000000a04a58762cdf594616b5875945de5b0dc3ad7ee08749940bf130b7d3.bin"
                .to_string()
        )?);

        println!("{}", merkle_proof.to_string_format());

        Ok(())
    }
}
