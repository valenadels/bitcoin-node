use crate::{constants::SAVED_ACCOUNTS, node_error::NodeError};

use std::{fs::File, io::Write};

/// Represents information about an account.
#[derive(Clone)]
pub struct AccountInfo {
    /// The name of the account.
    pub name: String,
    /// The private key associated with the  account.
    pub private_key: String,
    /// The Bitcoin address associated with the account.
    pub bitcoin_address: String,
}

impl AccountInfo {
    /// Creates a new `WalletInfo` struct by parsing a string representation of  account information.
    ///
    /// # Arguments
    ///
    /// * `info` - The string representation of wallet information in the format:
    ///                   "<bitcoin_address>;<private_key>;<name>"
    ///
    /// # Returns
    ///
    /// Returns a new `WalletInfo` struct with the parsed values.
    pub fn new_from_string(info: String) -> Self {
        let substrings: Vec<String> = info.split(';').map(|s| s.to_string()).collect();
        AccountInfo {
            bitcoin_address: substrings[0].clone(),
            private_key: substrings[1].clone(),
            name: substrings[2].clone(),
        }
    }

    /// Creates a new `AccountInfo` struct with the specified values.
    pub fn new_from_values(bitcoin_address: String, private_key: String, name: String) -> Self {
        AccountInfo {
            bitcoin_address,
            private_key,
            name,
        }
    }
    /// Creates a new `AccountInfo` struct by parsing a string representation of account information.
    pub fn to_string_format(&self) -> String {
        format!(
            "{};{};{}",
            self.bitcoin_address, self.private_key, self.name
        )
    }
    /// Saves the `AccountInfo` struct to a file.
    pub fn save_to_file(&self) -> Result<(), NodeError> {
        let info = self.to_string_format();
        let mut file = File::options()
            .write(true)
            .append(true)
            .open(SAVED_ACCOUNTS)
            .map_err(|_| NodeError::FailedToOpenFile("Failed to open file block".to_string()))?;

        file.write(info.as_bytes())
            .map_err(|_| NodeError::FailedToWrite("Failed to write file block".to_string()))?;
        file.write("\n".as_bytes())
            .map_err(|_| NodeError::FailedToWrite("Failed to write file block".to_string()))?;

        Ok(())
    }
    /// Extracts the name of the wallet.
    pub fn extract_name(&self) -> String {
        self.name.clone()
    }
    /// Extracts the private key of the account.
    pub fn extract_private_key(&self) -> String {
        self.private_key.clone()
    }
    /// Extracts the Bitcoin address of the account.
    pub fn extract_bitcoin_address(&self) -> String {
        self.bitcoin_address.clone()
    }

    /// Creates a copy of the `AccountInfo` struct.
    pub fn copy(&self) -> Self {
        AccountInfo {
            bitcoin_address: self.bitcoin_address.clone(),
            private_key: self.private_key.clone(),
            name: self.name.clone(),
        }
    }
}
