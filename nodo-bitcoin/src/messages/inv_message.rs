use std::io::Read;

use crate::{
    block::block_hash::BlockHash,
    constants::{INVENTORY_LENGTH, MAX_INVENTORY_VECTOR, MSG_BLOCK},
    node_error::NodeError,
    utils::Utils,
};

/// Represents an "inv" message in the Bitcoin peer-to-peer network.
///
/// The "inv" message transmits one or more inventories of objects known to the transmitting peer. It can be sent unsolicited
/// to announce new transactions or blocks, or it can be sent in reply to a "getblocks" message or "mempool" message.
///
/// The receiving peer can compare the inventories from an "inv" message against the inventories it has already seen, and
/// then use a follow-up message to request unseen objects.
///
/// # Fields
///
/// * `count` - The number of inventory entries.
/// * `inventory` - One or more inventory entries up to a maximum of 50,000 entries.
#[derive(Debug, PartialEq)]
pub struct InvMessage {
    count: u8,
    pub inventory: Vec<InventoryEntry>,
}

/// Represents an entry in the inventory list of an "inv" message.
///
/// # Fields
///
/// * `inv_type` - The type of object being inventoried (e.g., transaction or block).
/// * `hash` - The hash of the object being inventoried.
impl InvMessage {
    pub fn new(count: u64, inv_type: u32, hash: [u8; 32]) -> Result<Self, NodeError> {
        Ok(Self {
            count: count.try_into().map_err(|_| {
                NodeError::FailedToConvert("Failed to convert into count".to_string())
            })?,
            inventory: vec![InventoryEntry { inv_type, hash }],
        })
    }

    /// Converts 'bytes' to an Inv message.
    /// # Arguments
    /// * `bytes` - A byte array with the message payload.
    /// # Returns
    /// * `Result<Inv, NodeError>` - A new Inv message or NodeError in case of error.
    pub fn from_bytes(bytes: &[u8]) -> Result<InvMessage, NodeError> {
        let mut offset = 0;

        // Read the count of block headers as a variable-length integer
        let (count, bytes_read) = Utils::read_varint(&bytes[offset..])?;
        offset += bytes_read;

        if count > MAX_INVENTORY_VECTOR {
            return Err(NodeError::InvalidSizeOfField(
                "The count is greater than the maximum allowed".to_string(),
            ));
        }

        let mut inventory_entries = vec![];
        for _ in 0..count {
            let inventory = &bytes[offset..(offset + INVENTORY_LENGTH)];
            inventory_entries.push(InventoryEntry::from_bytes(inventory).map_err(|_| {
                NodeError::FailedToRead("Failed to read Inv entry bytes".to_string())
            })?);
            offset += INVENTORY_LENGTH;
        }

        Ok(InvMessage {
            count: count.try_into().map_err(|_| {
                NodeError::FailedToConvert("Failed to convert into count".to_string())
            })?,
            inventory: inventory_entries,
        })
    }

    /// Converts an Inv message to a byte array.
    /// # Arguments
    /// * `self` - A reference to the Inv message.
    /// # Returns
    /// * `Result<Vec<u8>, NodeError>` - A byte array with the message payload or NodeError in case of error.
    pub fn to_bytes(&self) -> Result<Vec<u8>, NodeError> {
        let mut bytes = vec![];
        bytes.extend(&self.count.to_le_bytes());
        for entry in &self.inventory {
            bytes.extend(&entry.to_bytes().map_err(|_| {
                NodeError::FailedToWrite("Failed to extend Inv entry bytes".to_string())
            })?);
        }

        Ok(bytes)
    }
    /// Returns a reference to the block hash from the inventory.
    ///
    /// This function retrieves the block hash from the first element of the inventory and
    /// returns it as a reference. The function checks if the inventory type of the first element
    /// is `MSG_BLOCK` and returns an error of type `NodeError::InvalidType` if it is not.
    ///
    /// # Errors
    ///
    /// Returns an `Err(NodeError::InvalidType)` if the first element of the inventory is not of type `MSG_BLOCK`.
    pub fn block_hash(&self) -> Result<&BlockHash, NodeError> {
        if self.inventory[0].inv_type != MSG_BLOCK {
            return Err(NodeError::InvalidType(
                "Invalid type, not a block inventory".to_string(),
            ));
        }

        Ok(&self.inventory[0].hash)
    }
}

#[derive(Debug, PartialEq)]
/// Represents an entry in the inventory list of an "inv" message.
/// # Fields
/// * `inv_type` - The type of object being inventoried:
/// 0 ERROR Any data of with this number may be ignored
/// 1 MSG_TX Hash is related to a transaction
/// 2 MSG_BLOCK Hash is related to a data block
/// 3 MSG_FILTERED_BLOCK Hash of a block header; identical to MSG_BLOCK. Only to be used in getdata message. Indicates the reply should be a merkleblock message rather than a block message; this only works if a bloom filter has been set. See BIP 37 for more info.
/// 4 MSG_CMPCT_BLOCK Hash of a block header; identical to MSG_BLOCK. Only to be used in getdata message. Indicates the reply should be a cmpctblock message. See BIP 152 for more info.
/// 0x40000001 MSG_WITNESS_TX Hash of a transaction with witness data. See BIP 144 for more info.
/// 0x40000002 MSG_WITNESS_BLOCK Hash of a block with witness data. See BIP 144 for more info.
/// 0x40000003 MSG_FILTERED_WITNESS_BLOCK Hash of a block with witness data. Only to be used in getdata message. Indicates the reply should be a merkleblock message rather than a block message; this only works if a bloom filter has been set. See BIP 144 for more info.
/// * `hash` - The hash of the object being inventoried.
pub struct InventoryEntry {
    pub inv_type: u32,
    pub hash: [u8; 32],
}

impl InventoryEntry {
    /// Converts 'bytes' to an InventoryEntry.
    /// # Arguments
    /// * `bytes` - A byte array with the message payload.
    /// # Returns
    /// * `Result<InventoryEntry, NodeError>` - A new InventoryEntry or NodeError in case of error.
    pub fn from_bytes(bytes: &[u8]) -> Result<InventoryEntry, NodeError> {
        let mut inv_type_bytes = [0u8; 4];
        let mut hash_bytes = [0u8; 32];
        let mut cursor = std::io::Cursor::new(bytes);
        cursor
            .read_exact(&mut inv_type_bytes)
            .map_err(|_| NodeError::FailedToRead("Failed to read Inv entry bytes".to_string()))?;
        cursor
            .read_exact(&mut hash_bytes)
            .map_err(|_| NodeError::FailedToRead("Failed to read Inv entry bytes".to_string()))?;
        Ok(InventoryEntry {
            inv_type: u32::from_le_bytes(inv_type_bytes),
            hash: hash_bytes,
        })
    }

    /// Converts an InventoryEntry to a byte array.
    /// # Arguments
    /// * `self` - A reference to the InventoryEntry.
    /// # Returns
    /// * `Result<Vec<u8>, NodeError>` - A byte array with the message payload or NodeError in case of error.
    pub fn to_bytes(&self) -> Result<Vec<u8>, NodeError> {
        let mut bytes = vec![];
        bytes.extend(&self.inv_type.to_le_bytes());
        bytes.extend(&self.hash);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes_inv() {
        let entry1 = InventoryEntry {
            inv_type: 1,
            hash: [0u8; 32],
        };
        let entry2 = InventoryEntry {
            inv_type: 2,
            hash: [0u8; 32],
        };
        let inv = InvMessage {
            count: 2,
            inventory: vec![entry1, entry2],
        };
        let expected_bytes = [
            0x02, // count
            0x01, 0x00, 0x00, 0x00, // entry.inv_type
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x02, 0x00, 0x00, 0x00, // entry.inv_type
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
        ];
        assert_eq!(inv.to_bytes().unwrap(), expected_bytes);
    }

    #[test]
    fn test_from_bytes_inv() {
        let expected_entry1 = InventoryEntry {
            inv_type: 1,
            hash: [0u8; 32],
        };
        let expected_entry2 = InventoryEntry {
            inv_type: 2,
            hash: [0u8; 32],
        };
        let expected_inv = InvMessage {
            count: 2,
            inventory: vec![expected_entry1, expected_entry2],
        };
        let bytes = [
            0x02, // count
            0x01, 0x00, 0x00, 0x00, // entry.inv_type
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x02, 0x00, 0x00, 0x00, // entry.inv_type
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
        ];
        assert_eq!(InvMessage::from_bytes(&bytes).unwrap(), expected_inv);
    }
    #[test]
    fn test_inventory_entry_to_bytes() {
        let entry = InventoryEntry {
            inv_type: 1,
            hash: [0u8; 32],
        };
        let expected_bytes = [
            0x01, 0x00, 0x00, 0x00, // entry.inv_type
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
        ];
        assert_eq!(entry.to_bytes().unwrap(), expected_bytes.to_vec());
    }

    #[test]
    fn test_inventory_entry_from_bytes() {
        let expected_entry = InventoryEntry {
            inv_type: 1,
            hash: [0u8; 32],
        };
        let bytes = [
            0x01, 0x00, 0x00, 0x00, // entry.inv_type
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
            0x00, 0x00, 0x00, 0x00, // entry.hash
        ];
        assert_eq!(InventoryEntry::from_bytes(&bytes).unwrap(), expected_entry);
    }
}
