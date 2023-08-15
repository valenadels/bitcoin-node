use gtk::{prelude::*, Builder, Fixed as GtkFixed, TreeStore, TreeView, Widget};

use crate::{
    block::proof_of_inclusion::MerkleProof,
    node_error::NodeError,
    ui::utils::{get_object_by_name, u8_to_hex_string},
    wallet::account::Account,
};

/// The transactions page.
/// This page is used to display the UTXO of the current account.
pub struct TransactionsPage {
    /// The page itself.
    pub page: GtkFixed,
    /// The transactions store
    txs_store: TreeStore,
}

impl TransactionsPage {
    /// Creates a new transactions page.
    /// # Arguments
    /// * `child` - The child widget.
    /// * `builder` - The builder used to create the page.
    /// # Returns
    /// * The transactions page.
    /// # Errors
    /// NodeError::UIError if the child widget could not be downcast to a GtkFixed.
    pub fn new(child: Widget, builder: Builder) -> Result<Self, NodeError> {
        let page = child
            .downcast::<GtkFixed>()
            .map_err(|_| NodeError::UIError("Failed to downcast to GtkFixed".to_string()))?;

        let txs_tree_view: TreeView = get_object_by_name(&builder, "utxo_tree_view")?;

        let txs_store = match txs_tree_view.model() {
            Some(model) => model,
            None => return Err(NodeError::UIError("Failed to get model".to_string())),
        }
        .downcast::<TreeStore>()
        .map_err(|_| NodeError::UIError("Invalid model type: txs_store".to_string()))?;

        Ok(TransactionsPage { page, txs_store })
    }

    /// Builds the transactions list.
    /// # Arguments
    /// * `account` - The account to build the transactions list for.
    /// # Returns
    /// * Result<(), NodeError> - The result of the operation.
    pub fn build_transactions_list(&self, account: &Account) -> Result<(), NodeError> {
        let transactions = account.copy().utxo_set.set;
        for transaction in transactions {
            let iter = self.txs_store.append(None);
            let iter_child = self.txs_store.append(Some(&iter));
            let mut tx_id = transaction.0;
            tx_id.reverse();
            let tx_id_text = u8_to_hex_string(&tx_id);

            let mut total_amount: f64 = 0.0;
            for output in transaction.1.clone() {
                total_amount += output.value();
            }

            let block_path = transaction.1[0].block_path.clone();
            let proof_result =
                match MerkleProof::path_for_tx_in_block(u8_to_hex_string(&tx_id), block_path) {
                    Ok(proof_of_inclusion) => proof_of_inclusion,
                    Err(e) => format!("Not found - Error: {:?}", e),
                };
            self.txs_store.set_value(&iter, 0, &tx_id_text.to_value());
            self.txs_store.set_value(&iter, 1, &total_amount.to_value());
            self.txs_store
                .set_value(&iter_child, 0, &proof_result.to_value());
        }

        self.page.show_all();

        Ok(())
    }

    /// Clears the transactions list in the UI.
    ///
    /// This function removes all the rows from the `list_box_transactions` ListBox in the UI.
    /// It iterates over each row in the list and removes it using the `remove` method of the ListBox.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the list is successfully cleared.
    /// Returns `Err(NodeError::UIError)` if the `list_box_transactions` object is not found in the builder.
    pub fn clear_transactions_list(&self) -> Result<(), NodeError> {
        self.txs_store.clear();
        Ok(())
    }

    /// Clears the transactions list in the UI and builds it again with the transactions of the given account.
    /// # Arguments
    /// * `account` - The account to build the transactions list for.
    /// # Returns
    /// * Result<(), NodeError> - The result of the operation.
    pub fn clear_and_build_txs_list(&self, account: &Account) -> Result<(), NodeError> {
        self.clear_transactions_list()?;
        self.build_transactions_list(account)
    }
}
