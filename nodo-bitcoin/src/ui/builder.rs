use crate::constants::NO_ARGS_LEN;
use crate::node::run_node;
use crate::node_error::NodeError;
use crate::transactions::transaction::Transaction;
use crate::wallet::account::Account;
use crate::wallet::wallet_account_info::AccountInfo;
use std::sync::mpsc;
use std::thread;

use super::components::transactions_confirmed_data::{Amount, TransactionConfirmedData};
use super::pages::accounts_page::AccountsPage;
use super::pages::add_account_page::AddAccountPage;
use super::pages::block_explorer_page::BlockExplorerPage;
use super::pages::main_window::MainWindow;
use super::pages::overview_page::OverviewPage;
use super::ui_message::UIMessage;
use super::utils::read_saved_wallet_and_accounts_from_file;
use glib::{Receiver, Sender};
use gtk::gdk::Screen;
use gtk::gio::ApplicationFlags;
use gtk::{prelude::*, StyleContext};
use gtk::{Application, Builder};

/// Loads the CSS file from the resources folder and applies it to the application.
fn load_css() {
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(include_bytes!("css/style.css"))
        .expect("Failed to load CSS");
    StyleContext::add_provider_for_screen(
        &Screen::default().expect("Could not connect to a screen"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

/// Builds the UI from the glade file and connects the signals to the handlers.
/// The UI is built on the main thread and the node and wallet run on a separate thread.
/// The UI thread and the node thread communicate via channels.
/// The UI thread and the wallet thread communicate via channels.
/// # Arguments
/// * `app` - The GTK application.
/// # Returns
/// * `Result<(), NodeError>` - Returns `Ok(())` if the UI was built successfully.
fn build_ui(app: &Application) -> Result<(), NodeError> {
    let glade_src = include_str!("window.glade");
    let builder: Builder = Builder::from_string(glade_src);
    let (wallet_node_sender, ui_reciever): (Sender<UIMessage>, Receiver<UIMessage>) =
        glib::MainContext::channel(glib::Priority::default());
    let (ui_sender, wallet_receiver) = mpsc::channel();
    let (main_window, account_page) = build_windows(app, builder.clone(), &ui_sender)?;
    let accounts_page =
        build_account_page(&main_window, ui_sender.clone(), &account_page, builder)?;

    handle_ui_messages(ui_reciever, main_window, ui_sender, accounts_page);

    thread::spawn(move || run_node(wallet_node_sender, wallet_receiver));
    Ok(())
}

/// Builds the windows from the glade file
/// # Arguments
/// * `app` - The GTK application.
/// * `builder` - The GTK builder.
/// * `ui_sender` - The channel sender for sending messages to the UI thread.
/// * `wallet_node_sender` - The channel sender for sending messages to the wallet thread.
/// # Returns
/// * `Result<(OverviewPage, MainWindow, BlockExplorerPage, AddAccountPage, TransactionsPage), NodeError>` - Returns a tuple containing the windows.
fn build_windows(
    app: &Application,
    builder: Builder,
    ui_sender: &mpsc::Sender<UIMessage>,
) -> Result<(MainWindow, AddAccountPage), NodeError> {
    let main_window = MainWindow::build(app, builder.clone(), ui_sender)?;
    let account_page = AddAccountPage::build(app, builder, ui_sender.clone(), &main_window)?;
    Ok((main_window, account_page))
}

/// Shows the main window according to the saved accounts. If no accounts are saved, the overview page is shown
/// and the user can add an account. If there are saved accounts, the main window is shown and the accounts are
/// added to the accounts list. The main account is the first account in the list.
/// # Arguments
/// * `main_window` - The main window.
/// * `ui_sender` - The channel sender for sending messages to the UI thread.
/// * `account_page` - The add account page.
/// # Returns
/// * `Result<(), NodeError>` - Returns `Ok(())` if the login window or main window was shown successfully.
fn build_account_page(
    main_window: &MainWindow,
    ui_sender: mpsc::Sender<UIMessage>,
    account_page: &AddAccountPage,
    builder: Builder,
) -> Result<AccountsPage, NodeError> {
    let saved_accounts = read_saved_wallet_and_accounts_from_file()?;
    let accounts_page =
        AccountsPage::build(builder, &ui_sender, saved_accounts.clone(), account_page)?;
    if saved_accounts.is_empty() {
        main_window.set_wallet_name("Login into your account")?;
        accounts_page.no_accounts_saved()?;
    } else {
        ui_sender
            .send(UIMessage::AddAccountsFromAppStart(saved_accounts.clone()))
            .map_err(|_| {
                NodeError::FailedToSendMessage(
                    "Failed to send add account message to UI thread".to_string(),
                )
            })?;
        accounts_page.show_current_account_info(&saved_accounts[0])?;
        main_window.set_wallet_name(saved_accounts[0].name.as_str())?;
    }

    main_window.show();
    Ok(accounts_page)
}

/// Handles the UI messages received from the node/wallet thread.
/// # Arguments
/// * `ui_reciever` - The channel receiver for receiving messages from the UI thread.
/// * `block_explorer` - The block explorer page.
/// * `overview_page` - The overview page.
/// * `main_window` - The main window.
/// * `account_page` - The add account page.
/// * `ui_sender` - The channel sender for sending messages to the UI thread.
/// * `transactions_page` - The transactions page.
fn handle_ui_messages(
    ui_reciever: Receiver<UIMessage>,
    mut main_window: MainWindow,
    ui_sender: mpsc::Sender<UIMessage>,
    accounts_page: AccountsPage,
) {
    ui_reciever.attach(None, move |message| {
        match message {
            UIMessage::StartingDate(timestamp) => {
                main_window.block_explorer_page.set_starting_date(timestamp);
            }
            UIMessage::TotalBlocksToDownload(n_blocks) => {
                main_window.block_explorer_page.set_total_blocks(n_blocks);
            }
            UIMessage::UpdateBlocksProgress => {
                main_window.block_explorer_page.increment_progress_bar();
            }
            UIMessage::InitialBlockHeaders(block_headers) => {
                build_block_list(&main_window.block_explorer_page, block_headers);
            }
            UIMessage::AddNewAccount(account, account_info) => {
                add_account_to_list(&accounts_page, account_info.clone(), ui_sender.clone());
                change_account(account, &main_window, account_info, &accounts_page);
            }
            UIMessage::NewCurrentAccount(account, account_info) => {
                change_account(account, &main_window, account_info, &accounts_page);
            }
            UIMessage::NewTransactionReceived(transaction, amount) => {
                show_new_tx(transaction, &amount, &main_window.overview_page);
                update_received_amount(&main_window.overview_page, amount);
            }
            UIMessage::NewTransactionSent(transaction, amount) => {
                show_new_tx(transaction, &amount, &main_window.overview_page);
                update_spent_amount(&main_window.overview_page, amount);
            }
            UIMessage::NewTransactionsConfirmed(transacion_data) => {
                update_on_transaction_confirmed(transacion_data, &main_window);
            }
            UIMessage::NotificationMessage(message) => {
                main_window.overview_page.show_new_tx_alert(message);
            }
            UIMessage::NewBlock(block) => {
                main_window
                    .block_explorer_page
                    .add_new_block_received(block);
            }
            UIMessage::UpdateHeadersProgress => {
                main_window.block_explorer_page.show_loading_headers();
            }
            UIMessage::HeadersDownloadFinished => {
                main_window.block_explorer_page.hide_loading_headers();
            }
            _ => {
                println!("Message not handled");
            }
        }
        glib::Continue(true)
    });
}

/// Updates the overview page when a new transaction is received
/// # Arguments
/// * `overview_page` - The overview page.
/// * `amount` - The amount of the transaction.
fn update_received_amount(overview_page: &OverviewPage, amount: Amount) {
    overview_page
        .update_pending_to_receive_amount(&amount)
        .unwrap_or_else(|e| {
            println!("Failed to update pending tx amount: {:?}", e);
        });
}

/// Updates the overview page when a new transaction is sent.
/// # Arguments
/// * `overview_page` - The overview page.
/// * `amount` - The amount of the transaction.
fn update_spent_amount(overview_page: &OverviewPage, amount: Amount) {
    overview_page
        .update_pending_to_send_amount(&amount)
        .unwrap_or_else(|e| {
            println!("Failed to update pending tx amount: {:?}", e);
        });
}

/// Shows a new tx in the pending page
/// # Arguments
/// * `transaction` - The transaction to show.
/// * `amount` - The amount of the transaction.
/// * `overview_page` - The overview page.
fn show_new_tx(transaction: Transaction, amount: &Amount, overview_page: &OverviewPage) {
    overview_page
        .add_pending_transaction(&transaction, amount)
        .unwrap_or_else(|e| {
            println!("Failed to add pending transaction: {:?}", e);
        });
}

/// Updates the transactions list in the transactions page when a transaction is confirmed and updates the
/// account in the overview page.
/// # Arguments
/// * `account` - The account.
/// * `transactions_page` - The transactions page.
fn update_on_transaction_confirmed(tx_data: TransactionConfirmedData, main_window: &MainWindow) {
    main_window
        .transactions_page
        .clear_and_build_txs_list(&tx_data.account)
        .unwrap_or_else(|e| {
            println!("Failed to clear transactions list: {:?}", e);
        });
    main_window
        .overview_page
        .update_transactions_and_account(tx_data)
        .unwrap_or_else(|e| {
            println!("Failed to remove pending transaction: {:?}", e);
        });
    main_window.show();
}

/// Changes the current account and updates the UI.
/// # Arguments
/// * `account` - The new account.
/// * `transactions_page` - The transactions page.
/// * `overview_page` - The overview page.
fn change_account(
    account: Account,
    main_window: &MainWindow,
    account_info: AccountInfo,
    accounts_page: &AccountsPage,
) {
    main_window
        .transactions_page
        .clear_and_build_txs_list(&account)
        .unwrap_or_else(|e| {
            println!("Failed to clear transactions list: {:?}", e);
        });
    main_window
        .overview_page
        .update_pending_and_confirmed_transactions(account)
        .unwrap_or_else(|e| {
            println!(
                "Failed to update pending and confirmed transactions: {:?}",
                e
            );
        });
    main_window
        .set_wallet_name(account_info.name.as_str())
        .unwrap_or_else(|e| {
            println!("Failed to set wallet name: {:?}", e);
        });
    accounts_page
        .show_current_account_info(&account_info)
        .unwrap_or_else(|e| {
            println!("Failed to set new account info: {:?}", e);
        });
}

/// Adds an account to the main window and builds the accounts list.
///
/// # Arguments
///
/// * `main_window` - A reference to the `MainWindow` instance representing the main UI window.
/// * `accounts` - The account information to be added to the main window.
/// * `account_page` - A reference to the `AddAccountPage` instance representing the UI page for adding an account.
/// * `ui_sender` - A reference to the `mpsc::Sender<UIMessage>` for sending UI messages.
///
/// # Remarks
///
/// This function adds the `accounts` to the `main_window` and builds the accounts list using the `account_page` and `ui_sender`.

fn add_account_to_list(
    accounts_page: &AccountsPage,
    account_info: AccountInfo,
    ui_sender: mpsc::Sender<UIMessage>,
) {
    println!("Add account message received");
    accounts_page
        .add_account_to_list(account_info, ui_sender)
        .unwrap_or_else(|e| {
            println!("Failed to build accounts list: {:?}", e);
        });
}
/// Builds a block list in the block explorer UI page.
///
/// # Arguments
///
/// * `block_explorer` - A reference to the `BlockExplorerPage` instance representing the UI page for the block explorer.
/// * `block_headers` - The block headers to be displayed in the block list.
///
/// # Remarks
///
/// This function builds a block list in the `block_explorer` UI page using the provided `block_headers`.
fn build_block_list(
    block_explorer: &BlockExplorerPage,
    block_headers: Vec<crate::block_header::BlockHeader>,
) {
    block_explorer
        .build_list_box(block_headers)
        .unwrap_or_else(|_| println!("Failed to build block headers list"));
}

/// Runs the UI and also the node.
pub fn run_ui() {
    let app = Application::builder().build();

    app.connect_startup(|_| load_css());
    if std::env::args().len() > NO_ARGS_LEN {
        app.set_flags(ApplicationFlags::HANDLES_OPEN);
        app.connect_open(move |app: &Application, _files, _| {
            build_ui(app).unwrap_or_else(|e| println!("Failed to build UI: {:?}", e))
        });
    } else {
        app.connect_activate(move |app: &Application| {
            build_ui(app).unwrap_or_else(|e| println!("Failed to build UI: {:?}", e))
        });
    }

    app.run();
}
