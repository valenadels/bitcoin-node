use crate::wallet::{account::Account, transactions_spent_received::TransactionsSpentAndReceived};

pub type Amount = String;

/// The wallet confirmed txs to the ui and the block hash of the block that confirmed them
pub struct TransactionConfirmedData {
    /// The transactions
    pub txs: TransactionsSpentAndReceived,
    /// The account
    pub account: Account,
}

impl TransactionConfirmedData {
    /// Create a new TransactionConfirmedData
    pub fn new(txs: TransactionsSpentAndReceived, account: Account) -> Self {
        Self { txs, account }
    }
}
