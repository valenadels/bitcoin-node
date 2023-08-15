use std::sync::mpsc;

use glib::clone;
use gtk::{prelude::*, Application, Builder, Button, Entry, Window as GtkWindow};

use crate::{
    node_error::NodeError,
    ui::{
        ui_message::UIMessage::{self, AddAccount},
        utils::get_object_by_name,
    },
    wallet::wallet_account_info::AccountInfo,
};

use super::main_window::MainWindow;
/// AddAccountPage is the page that allows the user to add a new account to the wallet
pub struct AddAccountPage {
    /// window is the window that contains the page
    pub window: GtkWindow,
    /// builder to build the page
    pub builder: Builder,
}

impl AddAccountPage {
    /// Builds the page
    /// # Arguments
    /// * `app` - the application
    /// * `builder` - the builder to build the page
    /// * `ui_sender_to_wallet` - the sender to send messages to the wallet,
    /// * `ui_sender` - the sender to send messages to the UI
    /// * `main_window` - the main window
    /// # Returns
    /// * `Result<AddAccountPage, NodeError>` - the result of the build
    pub fn build(
        app: &Application,
        builder: Builder,
        ui_sender_to_wallet: mpsc::Sender<UIMessage>,
        main_window: &MainWindow,
    ) -> Result<Self, NodeError> {
        let account_window: GtkWindow = get_object_by_name(&builder, "account_window")?;
        account_window.set_application(Some(app));
        let new_account: Button = get_object_by_name(&builder, "new_account_add")?;
        let account_name: Entry = get_object_by_name(&builder, "new_account_account_name")?;
        let private_key: Entry = get_object_by_name(&builder, "new_account_private_key")?;
        let bitcoin_address: Entry = get_object_by_name(&builder, "new_account_bitcoin_address")?;
        let cloned_login = account_window.clone();

        let cloned_main_window = main_window.window.clone();
        new_account.connect_clicked(clone!(@weak bitcoin_address, @weak private_key, @weak account_name => move |_|{
            let new_account = AccountInfo::new_from_values(bitcoin_address.buffer().text(),private_key.buffer().text(), account_name.buffer().text());
            let _ = new_account.save_to_file();
            ui_sender_to_wallet.send(AddAccount(new_account.copy())).unwrap_or_else(|_| println!("Error sending AddAccount message to wallet"));
            cloned_login.set_visible(false);
            cloned_main_window.set_visible(true);
            account_name.set_text("");
            private_key.set_text("");
            bitcoin_address.set_text("");
        }));
        Ok(AddAccountPage {
            window: account_window,
            builder,
        })
    }
}
