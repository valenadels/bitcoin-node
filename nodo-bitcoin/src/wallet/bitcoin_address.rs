use crate::node_error::NodeError;

use super::account::Account;

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
/// Represents a bitcoin address.
pub struct BitcoinAddress {
    /// The address is a Vec<u8> that contains the address in bytes.
    pub address: Vec<u8>,
}

impl BitcoinAddress {
    /// Returns a BitcoinAddress from a String.
    pub fn from_string(address: &String) -> Result<BitcoinAddress, NodeError> {
        let address = bs58::decode(address)
            .into_vec()
            .map_err(|_| NodeError::FailedToParse("Failed to convert into vec".to_string()))?;

        Ok(BitcoinAddress { address })
    }

    /// Turns the Bitcoin Address into a string format.
    pub fn bs58_to_string(&self) -> String {
        bs58::encode(&self.address).into_string()
    }

    /// Takes the first byte and the last 4 out of the bitcoin address, the resulting Vec<u8> is the pk script
    pub fn to_pk_hash(bitcoin_address: &BitcoinAddress) -> Vec<u8> {
        let mut pk_hash = bitcoin_address.address.clone();
        pk_hash.remove(0);
        pk_hash.remove(pk_hash.len() - 1);
        pk_hash.remove(pk_hash.len() - 1);
        pk_hash.remove(pk_hash.len() - 1);
        pk_hash.remove(pk_hash.len() - 1);

        pk_hash
    }

    /// Converts a BitcoinAddress into a pk script.
    pub fn to_pk_script(bitcoin_address: &BitcoinAddress) -> Vec<u8> {
        let pk_hash = BitcoinAddress::to_pk_hash(bitcoin_address);

        Account::pk_hash_to_pk_script(&pk_hash)
    }
}
