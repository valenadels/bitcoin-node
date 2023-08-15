use gtk::{prelude::*, Builder, Button, Label, ListBox};
use std::sync::mpsc;

use crate::{
    node_error::NodeError,
    ui::{ui_message::UIMessage, utils::get_object_by_name},
    wallet::wallet_account_info::AccountInfo,
};

use super::add_account_page::AddAccountPage;
/// Shows the info of the accounts
pub struct AccountsPage {
    /// Builder
    pub builder: Builder,
    /// Account list
    pub accounts_list: ListBox,
    /// Label to show when no accounts are saved
    pub no_saved_accounts: Label,
    /// Shows the bitcoin address of the current account
    pub bitcoin_address_info: Label,
}

impl AccountsPage {
    /// Builds the page. It will build the accounts list if there are saved accounts and will show the info of the first account
    /// # Arguments
    /// * `builder` - Builder
    /// * `ui_sender_to_wallet` - Sender to wallet
    /// * `saved_accounts` - Saved accounts
    /// * `add_account_page` - Add account page
    /// # Returns
    /// * `Result<AccountsPage, NodeError>` - Result
    pub fn build(
        builder: Builder,
        ui_sender_to_wallet: &mpsc::Sender<UIMessage>,
        saved_accounts: Vec<AccountInfo>,
        add_account_page: &AddAccountPage,
    ) -> Result<Self, NodeError> {
        let accounts_list: ListBox = get_object_by_name(&builder, "accounts_list")?;
        let no_saved_accounts: Label = get_object_by_name(&builder, "no_saved_accounts")?;
        let bitcoin_address_info: Label = get_object_by_name(&builder, "bitcoin_address_info")?;

        Self::build_accounts_list(
            saved_accounts,
            add_account_page,
            ui_sender_to_wallet.clone(),
            &builder,
            &accounts_list,
        )?;

        Ok(AccountsPage {
            builder,
            accounts_list,
            no_saved_accounts,
            bitcoin_address_info,
        })
    }
    /// Builds the accounts list
    /// # Arguments
    /// * `saved_accounts` - Saved accounts
    /// * `add_account_page` - Add account page
    /// * `ui_sender` - Sender to UI
    /// * `builder` - Builder
    /// * `accounts_list` - Accounts list
    /// # Returns
    /// * `Result<(), NodeError>` - Result
    fn build_accounts_list(
        saved_accounts: Vec<AccountInfo>,
        add_account_page: &AddAccountPage,
        ui_sender: mpsc::Sender<UIMessage>,
        builder: &Builder,
        accounts_list: &ListBox,
    ) -> Result<(), NodeError> {
        let add_account: Button = get_object_by_name(builder, "add_account")?;

        for account in saved_accounts {
            let account_row = gtk::ListBoxRow::new();
            let button = gtk::Button::with_label(&account.name);
            account_row.add(&button);
            accounts_list.add(&account_row);
            let builder = builder.clone();
            let cloned_sender = ui_sender.clone();
            button.connect_clicked(move |_| {
                let cloned_account = account.clone();
                cloned_sender
                    .send(UIMessage::AccountChanged(account.clone()))
                    .unwrap_or_else(|_| println!("Error sending change account"));
                let label: Label = match get_object_by_name(&builder, "wallet_name") {
                    Ok(label) => label,
                    Err(_) => return,
                };
                label.set_text(cloned_account.name.as_str());
            });
        }

        accounts_list.show_all();

        let cloned_window = add_account_page.window.clone();
        add_account.connect_clicked(move |_| {
            cloned_window.set_visible(true);
        });
        Ok(())
    }

    /// Shows the info of the current account
    /// # Arguments
    /// * `account` - Account info to show
    /// # Returns
    /// * `Result<(), NodeError>` - Result
    pub fn show_current_account_info(&self, account: &AccountInfo) -> Result<(), NodeError> {
        let bitcoin_address: Label = get_object_by_name(&self.builder, "bitcoin_address")?;
        bitcoin_address.set_text("Bitcoin Address");
        self.bitcoin_address_info
            .set_text(account.bitcoin_address.as_str());
        Ok(())
    }

    /// Shows the no accounts saved label
    /// # Returns
    /// * `Result<(), NodeError>` - Result
    pub fn no_accounts_saved(&self) -> Result<(), NodeError> {
        let no_saved_accounts: Label = get_object_by_name(&self.builder, "no_saved_accounts")?;
        no_saved_accounts.set_text("Oops! You don't have any saved accounts. Press the Add account button to add an account!");
        no_saved_accounts.show();
        Ok(())
    }

    /// Adds a wallet account to the main window.
    ///
    /// This function adds a `AccountInfo` to the main window's account list.
    /// It also connects a callback function to the account's button click event,
    /// which sends a `UIMessage::AccountChanged` via the `ui_sender` channel.
    /// Additionally, it updates the text of a label with the account's name.
    ///
    /// # Arguments
    ///
    /// * `account` - The `AccountInfo` to be added to the main window.
    /// * `ui_sender` - The `mpsc::Sender<UIMessage>` channel to send the UI message.
    ///
    /// # Returns
    ///
    /// Returns `Result<(), NodeError>` indicating success or an `NodeError` in case of failure.
    ///
    pub fn add_account_to_list(
        &self,
        account: AccountInfo,
        ui_sender: mpsc::Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        self.no_saved_accounts.set_text("");
        let account_row = gtk::ListBoxRow::new();
        let button = gtk::Button::with_label(&account.name);
        account_row.add(&button);
        self.accounts_list.add(&account_row);
        let builder = self.builder.clone();
        self.accounts_list.show_all();
        button.connect_clicked(move |_| {
            let cloned_account = account.clone();
            ui_sender
                .send(UIMessage::AccountChanged(account.clone()))
                .unwrap_or_else(|_| println!("Error sending change account"));
            let label: Label = match get_object_by_name(&builder, "wallet_name") {
                Ok(label) => label,
                Err(_) => return,
            };
            label.set_text(cloned_account.name.as_str());
        });
        Ok(())
    }
}
