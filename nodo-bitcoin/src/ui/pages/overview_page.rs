use gtk::{prelude::*, Builder, Fixed as GtkFixed, Label, TreeStore, TreeView, Widget};

use crate::{
    node_error::NodeError,
    transactions::transaction::Transaction,
    ui::{
        components::transactions_confirmed_data::{Amount, TransactionConfirmedData},
        utils::{get_object_by_name, u8_to_hex_string},
    },
    wallet::account::Account,
};

/// OverviewPage where the user can see the pending and confirmed transactions
pub struct OverviewPage {
    /// The page itself
    pub page: GtkFixed,
    /// The tree store where the pending transactions are stored
    pending_txs_store: TreeStore,
    /// The list store where the confirmed transactions are stored
    confirmed_txs_store: TreeStore,
    /// The label where the available transactions amount is shown
    available_amount: Label,
    /// The label where the transactions amount to receive is shown
    pending_receive_amount: Label,
    /// The label where the transactions amount to spent is shown
    pending_spent_amount: Label,
    /// Shows a message when a new transaction of the wallet is received
    new_transaction: Label,
}

impl OverviewPage {
    /// Creates a new OverviewPage
    /// # Arguments
    /// * `child` - The child widget
    /// * `builder` - The builder to get the elements from the glade file
    /// # Returns
    /// * `Result<OverviewPage, NodeError>` - The result with the OverviewPage
    pub fn new(child: Widget, builder: Builder) -> Result<Self, NodeError> {
        let page = child
            .downcast::<GtkFixed>()
            .map_err(|_| NodeError::UIError("Failed to downcast to GtkFixed".to_string()))?;

        let pending_txs_tree_view: TreeView =
            get_object_by_name(&builder, "pending_txs_tree_view")?;

        let pending_txs_store = match pending_txs_tree_view.model() {
            Some(model) => model,
            None => return Err(NodeError::UIError("Failed to get model".to_string())),
        }
        .downcast::<TreeStore>()
        .map_err(|_| NodeError::UIError("Invalid model type: pending_txs_store".to_string()))?;

        let confirmed_txs_tree_view: TreeView =
            get_object_by_name(&builder, "confirmed_txs_tree_view")?;

        let confirmed_txs_store = match confirmed_txs_tree_view.model() {
            Some(model) => model,
            None => return Err(NodeError::UIError("Failed to get model".to_string())),
        }
        .downcast::<TreeStore>()
        .map_err(|_| NodeError::UIError("Invalid model type: confirmed_txs_store".to_string()))?;

        let available_amount: Label =
            get_object_by_name(&builder, "available_transactions_amount")?;
        let pending_spent_amount: Label =
            get_object_by_name(&builder, "pending_spent_transactions_amount")?;
        let pending_receive_amount: Label =
            get_object_by_name(&builder, "pending_receive_transactions_amount")?;
        let new_tx: Label = get_object_by_name(&builder, "label_new_tx")?;
        new_tx.set_text("No new notifications");

        Ok(OverviewPage {
            page,
            pending_txs_store,
            confirmed_txs_store,
            available_amount,
            pending_spent_amount,
            pending_receive_amount,
            new_transaction: new_tx,
        })
    }

    /// Adds a pending transaction to the list box
    /// # Arguments
    /// * `transaction` - The transaction to add
    /// # Returns
    /// * `Result<(), NodeError>` - The result
    pub fn add_pending_transaction(
        &self,
        transaction: &Transaction,
        amount: &Amount,
    ) -> Result<(), NodeError> {
        let iter = self.pending_txs_store.append(None);
        let mut tx_id = transaction.tx_id();
        tx_id.reverse();
        let tx_id_text = &u8_to_hex_string(tx_id.as_slice());

        self.pending_txs_store
            .set_value(&iter, 0, &tx_id_text.to_value());
        self.pending_txs_store
            .set_value(&iter, 1, &amount.to_value());

        self.page.show_all();

        Ok(())
    }

    /// Adds a confirmed transaction to the tree view
    /// # Arguments
    /// * `transaction` - The transaction to add
    /// # Returns
    /// * `Result<(), NodeError>` - The result
    pub fn add_confirmed_transactions(
        &self,
        transactions: TransactionConfirmedData,
    ) -> Result<(), NodeError> {
        let bitcoin_address = transactions.account.bitcoin_address;
        let all_txs = transactions.txs.all_txs();
        for transaction in all_txs {
            let iter = self.confirmed_txs_store.append(None);
            let mut tx_id = transaction.tx_id();
            tx_id.reverse();
            let tx_id_text = &u8_to_hex_string(tx_id.as_slice());

            let amount = transaction.amount_received_by_address(&bitcoin_address);

            self.confirmed_txs_store
                .set_value(&iter, 0, &tx_id_text.to_value());
            self.confirmed_txs_store
                .set_value(&iter, 1, &amount.to_value());
        }

        self.page.show_all();

        Ok(())
    }

    /// Shows a message for 10s when a new transaction of the wallet is received
    /// # Arguments
    /// * `address` - The address of the wallet
    pub fn show_new_tx_alert(&self, message: String) {
        self.new_transaction.set_text(&message);

        self.new_transaction.set_visible(true);
        let label_clone = self.new_transaction.clone();
        glib::timeout_add_seconds_local(10, move || {
            label_clone.set_visible(false);
            label_clone.set_text("No new notifications");
            label_clone.set_visible(true);
            glib::Continue(true)
        });
    }

    /// Removes a pending transaction from the tree store
    /// # Arguments
    /// * `tx_id` - The transaction id to remove
    /// # Returns
    /// * `Result<(), NodeError>` - The result
    fn remove_pending_transactions(&self, txs: Vec<Transaction>) -> Result<(), NodeError> {
        let mut iters_to_remove = Vec::new();
        for tx in txs {
            let mut tx_id = tx.tx_id();
            tx_id.reverse();
            let num_rows = self.pending_txs_store.iter_n_children(None);

            let tx_id_text = &u8_to_hex_string(&tx_id);

            for i in 0..num_rows {
                if let Some(iter) = self.pending_txs_store.iter_nth_child(None, i) {
                    let value = self.pending_txs_store.value(&iter, 0);
                    if let Ok(tx_id_row) = value.get::<String>() {
                        if tx_id_row.as_str() == tx_id_text {
                            iters_to_remove.push(iter);
                            break;
                        }
                    }
                }
            }
        }

        for iter in iters_to_remove {
            self.pending_txs_store.remove(&iter);
        }

        Ok(())
    }

    /// Updates the transactions, removing the pending transactions
    /// # Arguments
    /// * `transactions` - The transactions id to update
    /// # Returns
    /// * `Result<(), NodeError>` - The result
    pub fn update_transactions_and_account(
        &self,
        mut txs_data: TransactionConfirmedData,
    ) -> Result<(), NodeError> {
        let all_txs = txs_data.txs.all_txs();
        self.remove_pending_transactions(all_txs)?;

        self.update_account(&mut txs_data.account);
        self.add_confirmed_transactions(txs_data)?;

        self.page.show_all();

        Ok(())
    }

    /// Update account information
    /// # Arguments
    /// * `account` - The account to update
    pub fn update_account(&self, account: &mut Account) {
        self.available_amount
            .set_text(&account.balance_for_user().to_string());
        self.pending_receive_amount
            .set_text(&account.unconfirmed_received_balance());
        self.pending_spent_amount
            .set_text(&account.unconfirmed_spent_balance());
        self.page.show_all();
    }

    /// Updates the pending receive amount
    /// # Arguments
    /// * `amount` - The amount to update
    pub fn update_pending_to_receive_amount(&self, amount: &Amount) -> Result<(), NodeError> {
        let current_amount = self
            .pending_receive_amount
            .text()
            .parse::<f64>()
            .map_err(|_| {
                NodeError::FailedToParse("Failed to parse pending amount to receive".to_string())
            })?;
        let new_amount = amount.parse::<f64>().map_err(|_| {
            NodeError::FailedToParse("Failed to parse old pending amount to receive".to_string())
        })?;
        self.pending_receive_amount
            .set_text(&(current_amount + new_amount).to_string());
        Ok(())
    }

    /// Updates the pending receive amount
    /// # Arguments
    /// * `amount` - The amount to update
    pub fn update_pending_to_send_amount(&self, amount: &Amount) -> Result<(), NodeError> {
        let current_amount = self
            .pending_spent_amount
            .text()
            .parse::<f64>()
            .map_err(|_| {
                NodeError::FailedToParse("Failed to parse pending amount to spent".to_string())
            })?;
        let new_amount = amount.parse::<f64>().map_err(|_| {
            NodeError::FailedToParse("Failed to parse old pending amount to spend".to_string())
        })?;
        self.pending_spent_amount
            .set_text(&(current_amount + new_amount).to_string());
        Ok(())
    }

    /// Clears the transactions from the main window.
    ///
    /// This function removes all transaction rows from both the confirmed and pending transaction lists.
    ///
    /// # Arguments
    ///
    /// * `self` - A reference to the current object.
    pub fn clear_transactions(&self) {
        self.pending_txs_store.clear();
        self.confirmed_txs_store.clear();
        self.page.show_all();
    }

    /// Updates the pending and confirmed transactions
    /// # Arguments
    /// * `pending_transactions` - The pending transactions
    /// * `confirmed_transactions` - The confirmed transactions
    /// # Returns
    /// * `Result<(), NodeError>` - The result
    pub fn update_pending_and_confirmed_transactions(
        &self,
        mut account: Account,
    ) -> Result<(), NodeError> {
        self.clear_transactions();
        self.update_account(&mut account);

        let bitcoin_address = account.bitcoin_address.clone();
        let spent = account.unconfirmed_transactions.spent.clone();

        for transaction in spent {
            let amount = transaction.amount_spent_by_account(&mut account);
            self.add_pending_transaction(&transaction, &amount.to_string())?;
        }

        for transaction in account.unconfirmed_transactions.received.iter() {
            let amount = transaction.amount_received_by_address(&bitcoin_address);
            self.add_pending_transaction(transaction, &amount.to_string())?;
        }

        let confirmed_txs =
            TransactionConfirmedData::new(account.confirmed_transactions.clone(), account);

        self.add_confirmed_transactions(confirmed_txs)?;

        Ok(())
    }
}
