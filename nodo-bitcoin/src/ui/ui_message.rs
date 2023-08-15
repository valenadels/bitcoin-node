use crate::{
    block_header::BlockHeader,
    transactions::transaction::Transaction,
    wallet::{account::Account, wallet_account_info::AccountInfo},
};

use super::components::transactions_confirmed_data::{Amount, TransactionConfirmedData};

/// The messages that are sent between the UI, and the node and wallet
pub enum UIMessage {
    /// The initial block headers are sent to the UI from the node (only the last 10k are shown)
    InitialBlockHeaders(Vec<BlockHeader>),
    /// The node sends the UI the total number of blocks to download
    TotalBlocksToDownload(i64),
    /// A new block was downloaded, so the UI needs to update the block explorer progress bar
    UpdateBlocksProgress,
    /// Login data: bitcoin address, private key, user name
    Login(AccountInfo),
    /// The UI asks for the starting date timestamp
    StartingDate(String),
    /// Wallet add account
    AddAccount(AccountInfo),
    /// Wallet add account from app start
    AddAccountsFromAppStart(Vec<AccountInfo>),
    /// Create a new transaction: base_address, target_address, amount
    CreateNewTransaction(String, f64, f64),
    /// The node sends the UI the new block hash
    NewBlock(BlockHeader),
    /// The node sends the UI the new transaction received and the amount
    NewTransactionReceived(Transaction, Amount),
    /// The node sends the UI the new transaction sent and the amount
    NewTransactionSent(Transaction, Amount),
    /// The wallet sends the UI the new transaction id and the amount
    NewTransactionsConfirmed(TransactionConfirmedData),
    /// The wallet sends the UI the new account selected
    AccountChanged(AccountInfo),
    /// The UI receive the new account selected from the wallet
    NewCurrentAccount(Account, AccountInfo),
    /// Add the new account to the UI
    AddNewAccount(Account, AccountInfo),
    /// The wallet sends this msg when a new tx is received or confirmed for the wallet's accounts
    NotificationMessage(String),
    /// Message to update the headers count
    UpdateHeadersProgress,
    /// Message to hide the headers count and show the block progress bar
    HeadersDownloadFinished,
}
