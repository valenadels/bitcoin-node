use crate::transactions::transaction::Transaction;
/// The `ReceivedDataFromPeers` enum represents the data received from peers in the listener pool.
/// Only transactions and blocks are of interest.
pub enum ReceivedDataFromPeers {
    BlockHash(Vec<u8>),
    Transaction(Transaction),
}
