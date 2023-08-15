use bitcoin_hashes::{sha256d, Hash};

use crate::{
    block::block_hash::BlockHash,
    constants::{GENESIS_BITS, GENESIS_NONCE, GENESIS_TIMESTAMP, LENGTH_BLOCK_HEADERS},
    node_error::NodeError,
};

use self::block_header_bytes::BlockHeaderBytes;

pub mod block_header_bytes;

/// Struct representing a block header.
/// # Fields
/// * `version` - Version of the block.
/// * `prev_blockhash` - Hash of the previous block in little endian.
/// * `merkle_root_hash` - Hash of the merkle root in little endian.
/// * `timestamp` - Timestamp of the block.
/// * `bits` - Bits of the block.
/// * `nonce` - Nonce of the block.
/// * `hash` - Hash of the block header.
/// # Remarks
/// * The block header is 80 bytes long.
/// * The block header is serialized in little endian.
#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_blockhash: BlockHash,
    pub merkle_root_hash: BlockHash,
    pub timestamp: u32,
    pub n_bits: u32,
    pub nonce: u32,
    pub hash: Vec<u8>,
}

/// Genesis block header, also known as block 0.
pub const GENESIS_BLOCK_HEADER: BlockHeader = BlockHeader {
    version: 1,
    prev_blockhash: [0; 32],
    merkle_root_hash: [0; 32],
    timestamp: GENESIS_TIMESTAMP,
    n_bits: GENESIS_BITS,
    nonce: GENESIS_NONCE,
    hash: Vec::new(),
};

impl BlockHeader {
    /// Creates a new block header.
    /// # Arguments
    /// * `version` - Version of the block.
    /// * `prev_blockhash` - Hash of the previous block in bytes.
    /// * `merkle_root_hash` - Hash of the merkle root in bytes.
    /// * `timestamp` - Timestamp of the block.
    /// * `bits` - Bits of the block.
    /// * `nonce` - Nonce of the block.
    /// # Returns
    /// * A new block header.
    pub fn new(
        version: i32,
        prev_blockhash: BlockHash,
        merkle_root_hash: BlockHash,
        timestamp: u32,
        n_bits: u32,
        nonce: u32,
        hash: Vec<u8>,
    ) -> Self {
        Self {
            version,
            prev_blockhash,
            merkle_root_hash,
            timestamp,
            n_bits,
            nonce,
            hash,
        }
    }

    /// Returns the hash of the block header.
    pub fn hash(&self) -> &Vec<u8> {
        &self.hash
    }

    /// Calculates the target threshold based on the `n_bits` value.
    ///
    /// # Returns
    ///
    /// A 256-bit array representing the target threshold.
    pub fn calculate_target_threshold(&self) -> BlockHash {
        let bytes_n_bits = self.n_bits.to_le_bytes();

        let exponent = bytes_n_bits[3];
        let mantisa = &bytes_n_bits[..3];

        let mut target_bytes = [0u8; 32];

        target_bytes[(32 - exponent + 2) as usize] = mantisa[0];
        target_bytes[(31 - exponent + 2) as usize] = mantisa[1];
        target_bytes[(30 - exponent + 2) as usize] = mantisa[2];

        target_bytes
    }

    /// Serialize a block header to a byte array
    ///
    /// # Returns
    ///
    /// * A byte array with the serialized block header
    pub fn to_bytes(&self) -> BlockHeaderBytes {
        let mut serialized_block_header = Vec::new();
        serialized_block_header.extend_from_slice(&self.version.to_le_bytes());
        serialized_block_header.extend(&self.prev_blockhash);
        serialized_block_header.extend(&self.merkle_root_hash);
        serialized_block_header.extend_from_slice(&self.timestamp.to_le_bytes());
        serialized_block_header.extend_from_slice(&self.n_bits.to_le_bytes());
        serialized_block_header.extend_from_slice(&self.nonce.to_le_bytes());
        serialized_block_header
    }

    /// Deserialize a block header from a byte array.
    ///
    /// # Arguments
    ///
    /// * `serialized_block_header` - A byte array with the serialized block header
    ///
    /// # Returns
    ///
    /// * A Result with a BlockHeader or a NodeError
    ///
    /// # Errors
    ///
    /// * NodeError - InvalidBlockHeaderLength | InvalidBlockHeaderField | InvalidBlockHeaderHash | InvalidBlockHeaderTargetThreshold
    ///             | InvalidBlockHeaderBits | InvalidBlockHeaderTimestamp
    ///
    pub fn from_bytes(serialized_block_header: &BlockHeaderBytes) -> Result<Self, NodeError> {
        if serialized_block_header.len() != LENGTH_BLOCK_HEADERS {
            return Err(NodeError::InvalidBlockHeaderLength(
                serialized_block_header.len().to_string(),
            ));
        }

        let version = i32::from_le_bytes(
            serialized_block_header[0..4]
                .try_into()
                .map_err(|_| NodeError::InvalidBlockHeaderField("Invalid version".to_string()))?,
        );
        let prev_blockhash = serialized_block_header[4..36].try_into().map_err(|_| {
            NodeError::InvalidBlockHeaderField("Invalid prev_blockhash".to_string())
        })?;
        let merkle_root_hash = serialized_block_header[36..68].try_into().map_err(|_| {
            NodeError::InvalidBlockHeaderField("Invalid merkle_root_hash".to_string())
        })?;
        let timestamp =
            u32::from_le_bytes(serialized_block_header[68..72].try_into().map_err(|_| {
                NodeError::InvalidBlockHeaderField("Invalid timestamp".to_string())
            })?);
        let bits = u32::from_le_bytes(
            serialized_block_header[72..76]
                .try_into()
                .map_err(|_| NodeError::InvalidBlockHeaderField("Invalid bits".to_string()))?,
        );
        let nonce = u32::from_le_bytes(
            serialized_block_header[76..80]
                .try_into()
                .map_err(|_| NodeError::InvalidBlockHeaderField("Invalid nonce".to_string()))?,
        );

        let hash = sha256d::Hash::hash(serialized_block_header)
            .to_byte_array()
            .to_vec();

        Ok(Self {
            version,
            prev_blockhash,
            merkle_root_hash,
            timestamp,
            n_bits: bits,
            nonce,
            hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{block_header::BlockHeader, node_error::NodeError};

    #[test]
    fn serialize_block_header() {
        let block_header = BlockHeader::new(
            1,
            [0; 32],
            [0; 32],
            1231006505,
            486604799,
            2083236893,
            [0; 32].to_vec(),
        );
        let serialized_block_header = block_header.to_bytes();
        assert_eq!(serialized_block_header.len(), 80);
        assert_eq!(serialized_block_header[0..4], [1, 0, 0, 0]);
        assert_eq!(serialized_block_header[4..36], [0; 32]);
        assert_eq!(serialized_block_header[36..68], [0; 32]);
        assert_eq!(serialized_block_header[68..72], [41, 171, 95, 73]);
        assert_eq!(serialized_block_header[72..76], [255, 255, 0, 29]);
        assert_eq!(serialized_block_header[76..80], [29, 172, 43, 124]);
    }

    #[test]
    fn deserialize_block_header() -> Result<(), NodeError> {
        let serialized_block_header = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 41, 171, 95, 73, 255, 255, 0, 29, 29, 172, 43, 124,
        ]
        .to_vec();

        let block_header = BlockHeader::from_bytes(&serialized_block_header)?;
        assert_eq!(block_header.version, 1);
        assert_eq!(block_header.prev_blockhash, [0; 32]);
        assert_eq!(block_header.merkle_root_hash, [0; 32]);
        assert_eq!(block_header.timestamp, 1231006505);
        assert_eq!(block_header.n_bits, 486604799);
        assert_eq!(block_header.nonce, 2083236893);
        Ok(())
    }

    #[test]
    fn deserialize_block_header_invalid_length() {
        let serialized_block_header = [0; 79].to_vec();
        let block_header = BlockHeader::from_bytes(&serialized_block_header);
        assert!(block_header.is_err());
    }

    #[test]
    fn test_get_target_threshold() -> Result<(), NodeError> {
        let n_bits: u32 = 0x181bc330;

        let block_header = BlockHeader::new(
            1,
            [0; 32],
            [0; 32],
            1231006505,
            n_bits,
            2083236893,
            [0; 32].to_vec(),
        );

        let target_threshold = block_header.calculate_target_threshold();

        let target_prueba = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1b, 0xc3, 0x30, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        assert_eq!(target_threshold, target_prueba);

        Ok(())
    }
}
