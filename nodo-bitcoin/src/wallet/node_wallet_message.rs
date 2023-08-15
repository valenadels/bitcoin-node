use crate::transactions::{transaction::Transaction, utxo_set::UtxoSet};

use super::wallet_account_info::AccountInfo;

/// The messages that are sent between the node and the wallet
pub enum NodeWalletMsg {
    /// The node sends the wallet a new transaction received from the network
    NewTransaction(Transaction),
    /// The node sends the wallet a new block file path
    NewBlock(String),
    /// The node sends the wallet the new account information
    CreateNewAccount(AccountInfo, UtxoSet),
}
