use glib::Sender;

use crate::{
    node_error::NodeError,
    transactions::{pk_script::PkScript, transaction::Transaction},
    ui::{components::transactions_confirmed_data::Amount, ui_message::UIMessage},
    utils::Utils,
};

use super::account::Account;

/// Represents the transactions sent and received by the user.
#[derive(Debug, Clone, Default)]
pub struct TransactionsSpentAndReceived {
    /// The transactions sent by the user.
    pub spent: Vec<Transaction>,
    /// The transactions received by the user.
    pub received: Vec<Transaction>,
}

impl TransactionsSpentAndReceived {
    /// Creates a new TransactionsSpentAndReceived struct.
    /// # Returns
    /// A new TransactionsSpentAndReceived struct with empty lists.
    pub fn new() -> Self {
        TransactionsSpentAndReceived {
            spent: Vec::new(),
            received: Vec::new(),
        }
    }

    /// Checks if the tx is contained in the transactions sent or received lists.
    /// # Arguments
    /// * `transaction` - The transaction to check.
    /// # Returns
    /// A boolean indicating if the tx is contained.
    pub fn contains(&self, tx: &Transaction) -> bool {
        let mut txs = self.spent.iter().chain(self.received.iter());
        txs.any(|tx_in| tx_in.tx_id() == tx.tx_id())
    }

    /// Adds a transaction to the transactions sent list
    /// # Arguments
    /// * `transaction` - The transaction to add.
    pub fn add_spent(&mut self, transaction: Transaction) {
        self.spent.push(transaction);
    }

    /// Adds a transaction to the transactions received list
    /// # Arguments
    /// * `transaction` - The transaction to add.
    pub fn add_received(&mut self, transaction: Transaction) {
        self.received.push(transaction);
    }

    /// Removes a transaction from a list of transactions
    /// # Arguments
    /// * `transaction` - The transaction to remove.
    /// * `list` - The list of transactions.
    /// # Returns
    /// A boolean indicating if the tx was removed.
    fn remove_tx(transaction: &Transaction, list: &mut Vec<Transaction>) -> bool {
        for (index, tx_in) in list.iter_mut().enumerate() {
            if tx_in.tx_id() == transaction.tx_id() {
                list.remove(index);
                return true;
            }
        }

        false
    }

    /// Removes a transaction from the transactions sent list
    /// # Arguments
    /// * `transaction` - The transaction to remove.
    /// # Returns
    /// A boolean indicating if the tx was removed.
    pub fn remove_spent(&mut self, transaction: &Transaction) -> bool {
        Self::remove_tx(transaction, &mut self.spent)
    }

    /// Removes a transaction from the transactions received list
    /// # Arguments
    /// * `transaction` - The transaction to remove.
    /// # Returns
    /// A boolean indicating if the tx was removed.
    pub fn remove_received(&mut self, transaction: &Transaction) -> bool {
        Self::remove_tx(transaction, &mut self.received)
    }

    /// Sends a message to the UI notifying that a transaction has been confirmed. It sends all the
    /// transactions sent and received.
    /// # Arguments
    /// * `ui_sender` - The sender channel to communicate with the UI.
    /// # Errors
    /// This function can return a `NodeError` in case that sending a message to the UI fails.
    pub fn send_confirmations_to_ui(&self, ui_sender: &Sender<UIMessage>) -> Result<(), NodeError> {
        let transactions = self.all_txs();
        for tx in transactions {
            let mut id = tx.tx_id();
            id.reverse();
            ui_sender
                .send(UIMessage::NotificationMessage(format!(
                    "Tx {} confirmed",
                    Utils::bytes_to_hex(&id)
                )))
                .map_err(|_| {
                    NodeError::FailedToSendMessage(
                        "Error sending notify confirmed tx message to UI".to_string(),
                    )
                })?;
        }

        Ok(())
    }

    /// Checks if a transaction output pk script is equal to the user's pk hash.
    /// # Arguments
    /// * `users_pk_hash` - The user's pk hash.
    /// * `pk_script` - The transaction output pk script.
    /// # Returns
    /// A boolean indicating if the pk script is equal to the user's pk hash.
    fn check_output_pk(users_pk_hash: &Vec<u8>, pk_script: &PkScript) -> bool {
        let tx_output_pk_hash = match Account::pk_script_to_pk_hash(pk_script) {
            Ok(pk_hash) => pk_hash,
            Err(_) => {
                println!("This is not a P2PKH script");
                return false;
            }
        };
        if users_pk_hash == &tx_output_pk_hash {
            return true;
        }

        false
    }

    /// Gets the received balance for a specific address.
    /// # Arguments
    /// * `users_pk_hash` - The user's pk hash.
    /// # Returns
    /// The received balance for the address.
    pub fn received_balance(&self, users_pk_hash: &Vec<u8>) -> Amount {
        let mut balance: f64 = 0.0;
        for tx in &self.received {
            for tx_output in &tx.tx_outputs {
                if Self::check_output_pk(users_pk_hash, &tx_output.pk_script) {
                    balance += tx_output.value();
                }
            }
        }
        balance.to_string()
    }

    /// Gets the spent balance for a specific account.
    /// # Arguments
    /// * `account` - The account to check.
    /// # Returns
    /// The spent amount for the account.
    pub fn spent_balance(&self, account: &mut Account) -> Amount {
        let mut input = 0.0;
        for tx in &self.spent {
            input += tx.amount_spent_by_account(account)
        }
        input.to_string()
    }

    /// # Returns
    /// A vector with all the transactions sent and received.
    pub fn all_txs(&self) -> Vec<Transaction> {
        let mut txs = self.spent.clone();
        txs.extend_from_slice(&self.received);
        txs
    }
}
