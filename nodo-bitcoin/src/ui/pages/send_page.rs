use std::sync::mpsc;

use crate::{
    node_error::NodeError,
    ui::{ui_message::UIMessage, utils::get_object_by_name},
};
use glib::clone;
use gtk::{prelude::*, Builder, Button, Entry, Fixed as GtkFixed, Widget};

/// Page to create new transactions
pub struct SendPage {
    /// The page itself
    pub page: GtkFixed,
    /// The builder to get the widgets from
    pub builder: Builder,
}

impl SendPage {
    /// Create a new send page
    /// # Arguments
    /// * `child` - The child widget
    /// * `builder` - The builder to get the widgets from
    /// * `ui_sender_to_wallet` - The sender to send messages to the wallet
    /// # Returns
    /// * `Result<SendPage, NodeError>` - The result of the function
    pub fn new(
        child: Widget,
        builder: Builder,
        ui_sender_to_wallet: mpsc::Sender<UIMessage>,
    ) -> Result<Self, NodeError> {
        let page = child
            .downcast::<GtkFixed>()
            .map_err(|_| NodeError::UIError("Failed to downcast to GtkFixed".to_string()))?;

        let send_transaction: Button = get_object_by_name(&builder, "send_transaction_button")?;
        let clear_all: Button = get_object_by_name(&builder, "clear_all")?;
        let fee: Entry = get_object_by_name(&builder, "send_fee")?;
        let bitcoin_address: Entry = get_object_by_name(&builder, "send_bitcoin_address")?;
        let amount_entry: Entry = get_object_by_name(&builder, "send_amount")?;

        send_transaction.connect_clicked(
            clone!(@weak fee, @weak bitcoin_address, @weak amount_entry => move |_| {
                let fee_num = fee.text().to_string().parse::<f64>().unwrap_or(0.0);
                let address_text = bitcoin_address.text().to_string();
                let amount = amount_entry
                    .text()
                    .to_string()
                    .parse::<f64>()
                    .unwrap_or(0.0);
                ui_sender_to_wallet
                    .send(UIMessage::CreateNewTransaction(
                        address_text,
                        amount,
                        fee_num,
                    ))
                    .unwrap_or_else(|e| {
                        println!("Error sending CreateNewTransaction message to wallet {}", e);
                    });
                fee.set_text("");
                bitcoin_address.set_text("");
                amount_entry.set_text("");

            }),
        );

        Self::clear_all(clear_all, fee, bitcoin_address, amount_entry);

        Ok(SendPage { page, builder })
    }

    /// Clears all the entries
    fn clear_all(clear_all: Button, fee: Entry, bitcoin_address: Entry, amount_entry: Entry) {
        clear_all.connect_clicked(move |_| {
            fee.set_text("");
            bitcoin_address.set_text("");
            amount_entry.set_text("");
        });
    }
}
