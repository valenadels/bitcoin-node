use std::sync::mpsc;

use super::block_explorer_page::BlockExplorerPage;
use super::overview_page::OverviewPage;
use super::send_page::SendPage;
use super::transactions_page::TransactionsPage;
use crate::node_error::NodeError;
use crate::ui::ui_message::UIMessage;
use crate::ui::utils::get_object_by_name;
use gtk::{prelude::*, Application, Builder, Label, Window as GtkWindow};

/// Main window of the application
pub struct MainWindow {
    /// Main window
    pub window: GtkWindow,
    /// Builder
    pub builder: Builder,
    /// Overview page of txs and balance
    pub overview_page: OverviewPage,
    /// Send page to create new txs
    pub send_page: SendPage,
    /// Block explorer page to see blocks
    pub block_explorer_page: BlockExplorerPage,
    /// Transactions page to see txs history
    pub transactions_page: TransactionsPage,
}

impl MainWindow {
    /// Creates a new main window
    /// # Arguments
    /// * `app` - Application
    /// * `builder` - Builder
    /// # Returns
    /// * `Result<MainWindow, NodeError>` - The main window
    pub fn build(
        app: &Application,
        builder: Builder,
        ui_sender_to_wallet: &mpsc::Sender<UIMessage>,
    ) -> Result<Self, NodeError> {
        let window: GtkWindow = get_object_by_name(&builder, "main_window")?;
        window
            .set_icon_from_file("src/ui/assets/icon.png")
            .map_err(|_| NodeError::UIError("Failed to set icon from file".to_string()))?;
        let (overview, send, block_explorer, transactions) =
            Self::build_navigation(&builder, ui_sender_to_wallet)?;
        window.set_application(Some(app));
        window.set_title("Inoxidables Node");
        Ok(MainWindow {
            window,
            builder,
            overview_page: overview,
            send_page: send,
            block_explorer_page: block_explorer,
            transactions_page: transactions,
        })
    }

    /// Sets the wallet name
    /// # Arguments
    /// * `wallet_name` - Wallet name
    /// # Returns
    /// * `Result<(), NodeError>` - Result
    pub fn set_wallet_name(&self, wallet_name: &str) -> Result<(), NodeError> {
        let label: Label = get_object_by_name(&self.builder, "wallet_name")?;
        label.set_text(wallet_name);
        Ok(())
    }

    /// Shows the main window
    /// # Arguments
    /// * `self` - The main window
    pub fn show(&self) {
        self.window.set_visible(true);
    }

    /// Builds the navigation pages
    /// # Arguments
    /// * `self` - The main window
    /// * `ui_sender_to_wallet` - Sender to wallet
    /// # Returns
    /// * `Result<(OverviewPage, SendPage, BlockExplorerPage, TransactionsPage), NodeError>` - The navigation pages
    pub fn build_navigation(
        builder: &Builder,
        ui_sender_to_wallet: &mpsc::Sender<UIMessage>,
    ) -> Result<(OverviewPage, SendPage, BlockExplorerPage, TransactionsPage), NodeError> {
        let overview = OverviewPage::new(
            get_object_by_name(builder, "overview_page")?,
            builder.clone(),
        )?;
        let send = SendPage::new(
            get_object_by_name(builder, "send_page")?,
            builder.clone(),
            ui_sender_to_wallet.clone(),
        )?;
        let block_explorer = BlockExplorerPage::new(
            get_object_by_name(builder, "block_explorer_page")?,
            builder.clone(),
        )?;
        let transactions = TransactionsPage::new(
            get_object_by_name(builder, "transactions_page")?,
            builder.clone(),
        )?;
        Ok((overview, send, block_explorer, transactions))
    }
}
