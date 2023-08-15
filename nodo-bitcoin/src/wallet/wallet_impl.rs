use std::{
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use super::{
    account::Account, bitcoin_address::BitcoinAddress, node_wallet_message::NodeWalletMsg,
    wallet_account_info::AccountInfo,
};

use glib::Sender;

use crate::{
    channels::wallet_channel::WalletChannel,
    node::broadcast_transaction,
    node_error::NodeError,
    transactions::{transaction::Transaction, utxo_set::UtxoSet},
    ui::{
        components::transactions_confirmed_data::TransactionConfirmedData, ui_message::UIMessage,
    },
};

use crate::wallet::node_wallet_message::NodeWalletMsg::NewBlock;
use crate::wallet::wallet_impl::NodeWalletMsg::CreateNewAccount;
use crate::wallet::wallet_impl::NodeWalletMsg::NewTransaction;

/// Represents a Wallet for the user.
pub struct Wallet {
    /// The wallet contains a list of accounts. The account at the head is the one that is
    /// being used.
    pub accounts: Vec<Account>,
    /// The list of blocks that have been checked by the wallet.
    checked_blocks: Vec<String>,
}

impl Wallet {
    /// Returns the balance for the given Bitcoin address in the UTXO set.
    pub fn balances_for_user(&self) -> Vec<f64> {
        let mut balances = Vec::new();

        for account in &self.accounts {
            let balance = account.balance_for_user();
            balances.push(balance);
        }

        balances
    }

    /// Returns the balance for the given Bitcoin Address in the UTXO set.
    pub fn balance_for_address(&self, address: &String) -> Result<f64, NodeError> {
        let account = self
            .accounts
            .iter()
            .find(|account| &account.bitcoin_address.bs58_to_string() == address)
            .ok_or(NodeError::AccountNotFound(
                "Account not found in wallet".to_string(),
            ))?;

        Ok(account.balance_for_user())
    }

    /// Returns a Wallet for the user.
    /// # Arguments
    /// * `utxo_set_arc` - The UTXO set to be used by the wallet, inside an Arc Mutex to be shared between threads.
    /// * `account_info` - The AccountInfo instance containing the Bitcoin address and private key for the user.
    /// * `ui_sender` - The Sender instance to be used to send messages to the UI.
    /// # Returns
    /// Returns a Result containing Ok(Wallet) if the wallet was created successfully, or a NodeError if an error occurs.
    pub fn initialize_wallet_for_user(
        utxo_set_arc: &Arc<Mutex<UtxoSet>>,
        account_info: &AccountInfo,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<Wallet, NodeError> {
        let bitcoin_address = account_info.extract_bitcoin_address();
        let private_key = account_info.extract_private_key();
        let utxo_lock = utxo_set_arc
            .lock()
            .map_err(|_| NodeError::FailedToSendMessage("Failed to lock utxo set".to_string()))?;

        let initial_account = Account::new(&utxo_lock, bitcoin_address, private_key)?;

        ui_sender
            .send(UIMessage::AddNewAccount(
                initial_account.clone(),
                account_info.clone(),
            ))
            .map_err(|_| {
                NodeError::FailedToSendMessage("Failed to send wallet created to ui".to_string())
            })?;

        Ok(Wallet {
            accounts: vec![initial_account],
            checked_blocks: Vec::new(),
        })
    }

    /// Returns a Wallet for the user.
    /// # Arguments
    /// * `utxo_set_arc` - The UTXO set to be used by the wallet, inside an Arc Mutex to be shared between threads.
    /// * `accounts_info` - The Vec<AccountInfo> instance containing the accounts saved in the file.
    /// * `ui_sender` - The Sender instance to be used to send messages to the UI.
    /// # Returns
    /// Returns a Result containing Ok(Wallet) if the wallet was created successfully, or a NodeError if an error occurs.
    pub fn initialize_wallet_with_saved_accounts(
        utxo_set_arc: &Arc<Mutex<UtxoSet>>,
        accounts_info: Vec<AccountInfo>,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<Wallet, NodeError> {
        let mut accounts = Vec::new();
        for account_info in accounts_info.clone() {
            let bitcoin_address = account_info.extract_bitcoin_address();
            let private_key = account_info.extract_private_key();
            let utxo_lock = utxo_set_arc.lock().map_err(|_| {
                NodeError::FailedToSendMessage("Failed to lock utxo set".to_string())
            })?;

            let account = Account::new(&utxo_lock, bitcoin_address, private_key)?;
            accounts.push(account);
        }

        ui_sender
            .send(UIMessage::NewCurrentAccount(
                accounts[0].copy(),
                accounts_info[0].clone(),
            ))
            .map_err(|_| {
                NodeError::FailedToChangeAccount("Failed to send account changed to ui".to_string())
            })?;

        Ok(Wallet {
            accounts,
            checked_blocks: Vec::new(),
        })
    }

    /// Adds an account to the wallet and sends the AddNewAccount message to the UI.
    /// # Arguments
    /// * `utxo_set` - The UTXO set to be used by the account.
    /// * `bitcoin_address` - The Bitcoin Address to be added.
    /// * `private_key` - The private key for the Bitcoin Address.
    /// * `wallet_node_sender` - The channel to send messages to the UI.
    /// # Returns
    /// Returns a Result containing Ok if the account was added and the message was sent successfully, or a NodeError if an error occurs.
    pub fn add_account(
        &mut self,
        utxo_set: &UtxoSet,
        account_info: AccountInfo,
        wallet_node_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        let bitcoin_address = account_info.extract_bitcoin_address();
        let private_key = account_info.extract_private_key();
        let new_account = Account::new(utxo_set, bitcoin_address, private_key)?;
        wallet_node_sender
            .send(UIMessage::AddNewAccount(new_account.copy(), account_info))
            .map_err(|_| {
                NodeError::FailedToSendMessage("Failed to send new account to ui".to_string())
            })?;
        self.accounts.push(new_account);
        Ok(())
    }

    /// Removes an account from the wallet.
    /// # Arguments
    /// * `bitcoin_address` - The Bitcoin Address to be removed.
    /// # Returns
    /// Returns a Result containing Ok if the account was removed successfully, or a NodeError if an error occurs.
    pub fn remove_account(&mut self, bitcoin_address: &String) -> Result<(), NodeError> {
        let bitcoin_address_to_remove = BitcoinAddress::from_string(&bitcoin_address.to_string())?;

        self.accounts
            .retain(|account| account.bitcoin_address != bitcoin_address_to_remove);

        Ok(())
    }

    /// Returns the Bitcoin Addresses for the user.
    /// # Returns
    /// Returns a vector containing the Bitcoin Addresses for the user.
    pub fn bitcoin_addresses(&self) -> Vec<BitcoinAddress> {
        let mut addresses = Vec::new();

        for account in &self.accounts {
            addresses.push(account.bitcoin_address());
        }

        addresses
    }

    /// Sends to the UI the NewTransactionSent or NewTransactionReceived message, depending on the
    /// type of transaction received from the node.
    /// # Arguments
    /// * `account` - The account to check.
    /// * `transaction` - The transaction received from the node.
    /// * `ui_sender` - The channel to send messages to the UI.
    /// # Returns
    /// Returns a Result containing Ok if the message was sent successfully, or a NodeError if an error occurs.
    pub fn send_new_transaction_info(
        account: &mut Account,
        transaction: &Transaction,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        for tx_input in transaction.tx_inputs.iter() {
            if account
                .utxo_set
                .contains_key(&tx_input.previous_output.tx_id)
            {
                let amount = transaction.amount_spent_by_account(account).to_string();
                ui_sender
                    .send(UIMessage::NewTransactionSent(transaction.clone(), amount))
                    .map_err(|_| {
                        NodeError::FailedToCreateTransaction(
                            "Failed send new transaction sent to ui".to_string(),
                        )
                    })?;
                return Ok(());
            }
        }
        ui_sender
            .send(UIMessage::NewTransactionReceived(
                transaction.clone(),
                transaction.amount_received_by_address(&account.bitcoin_address),
            ))
            .map_err(|_| {
                NodeError::FailedToCreateTransaction(
                    "Failed send new transaction sent to ui".to_string(),
                )
            })?;
        Ok(())
    }

    /// Receives a transaction incoming for the user and saves it in the wallet.
    /// The transaction is not yet included in a block.
    /// # Arguments
    /// * `tx` - The transaction to be received.
    /// * `address` - The address to which the transaction was sent.
    /// receiving the transaction.
    /// # Remarks
    /// This function is called when the node receives a transaction from the network.
    pub fn receive_incoming_transaction(
        &mut self,
        tx: Transaction,
        address: &BitcoinAddress,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        println!("Received incoming transaction, which is not yet included in a block, for address: {:?}", address.bs58_to_string());
        let current_account = self.current_account()?.clone();
        for account in self.accounts.iter_mut() {
            if &account.bitcoin_address == address
                && !account.unconfirmed_transactions.contains(&tx)
            {
                if &current_account.bitcoin_address == address {
                    Self::send_new_transaction_info(account, &tx, ui_sender)?;
                }
                account.add_new_unconfirmed_transaction(tx);
                ui_sender
                    .send(UIMessage::NotificationMessage(format!(
                        "Address {} received a new transaction",
                        account.bitcoin_address().bs58_to_string()
                    )))
                    .map_err(|_| {
                        NodeError::FailedToSendMessage(
                            "Error sending new tx from wallet account to UI".to_string(),
                        )
                    })?;

                break;
            }
        }
        Ok(())
    }

    /// Searches for the accounts stored in the wallet, for the one that matches the given address.
    /// # Arguments
    /// * `address` - The address to search for.
    /// # Returns
    /// Returns an `Option` containing a reference to the account if found, or `None` if not found.
    pub fn account_from_address(&self, address: String) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|account| account.bitcoin_address.bs58_to_string() == address)
    }

    /// Creates a new transaction from the specified base address to the target address
    /// with the given amount.
    ///
    /// # Arguments
    ///
    /// * `base_address` - The base address (sender's address) for the transaction.
    /// * `target_address_str` - The target address (receiver's address) for the transaction.
    /// * `amount` - The amount to be sent in the transaction.
    /// * `fee` - The fee to be paid for the transaction.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the created `Transaction` if successful, or an `Err`
    /// if the base address is not found in the wallet's accounts.
    pub fn create_transaction(
        &self,
        base_address: String,
        target_address_str: &String,
        amount: f64,
        fee: f64,
    ) -> Result<Transaction, NodeError> {
        println!(
            "Creating transaction from {} to {} of amount {}",
            base_address,
            target_address_str,
            amount - fee
        );
        let account = match self.account_from_address(base_address) {
            Some(account) => account,
            None => return Err(NodeError::AccountNotFound("Account not found".to_string())),
        };

        account.create_transaction(target_address_str, amount, fee)
    }

    /// Given a path of a new block, searches the unconfirmed txs of the wallet and removes
    /// the ones that are included in the block, adding them to the confirmed txs.
    /// It sends a message to the UI with the new confirmed txs for the current account.
    /// # Arguments
    /// * `path` - The path of the new block.
    /// * `ui_sender` - The channel to send messages to the UI.
    /// # Returns
    /// Returns a Result containing Ok if the transactions were confirmed successfully, or a NodeError if an error occurs.
    fn confirm_transactions(
        &mut self,
        path: &String,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        self.checked_blocks.push(path.to_string());
        let current_account = self.current_account()?.clone();
        for account in self.accounts.iter_mut() {
            let confirmed_transactions = account.confirm_transactions(path, ui_sender)?;
            if account.bitcoin_address() == current_account.bitcoin_address() {
                ui_sender
                    .send(UIMessage::NewTransactionsConfirmed(
                        TransactionConfirmedData::new(confirmed_transactions, account.clone()),
                    ))
                    .map_err(|_| {
                        NodeError::FailedToSendMessage("Error sending message to UI".to_string())
                    })?;
            }
        }

        Ok(())
    }

    /// Checks if the block has already been checked.
    pub fn has_block_been_checked(&self, block_path: &String) -> bool {
        self.checked_blocks.contains(block_path)
    }

    /// Updates the UTXO set of the accounts in the wallet.
    /// # Arguments
    /// * `block_path` - The path of the new block.
    /// # Returns
    /// Returns a Result containing Ok if the UTXO set was updated successfully, or a NodeError if an error occurs.
    pub fn update_accounts_utxo(&mut self, block_path: &String) -> Result<(), NodeError> {
        for account in self.accounts.iter_mut() {
            account.update_utxo(block_path)?;
        }

        Ok(())
    }

    /// Handles the communication between the wallet and the node.
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet instance.
    /// * `node_channel` - The channel to communicate with the node.
    /// * `ui_sender` - The channel to send messages to the UI.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the function completes successfully, or an `Err` if there was an error
    /// receiving messages from the node.
    pub fn handle_node_connection(
        wallet: Arc<Mutex<Wallet>>,
        node_channel: WalletChannel,
        ui_sender: Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        loop {
            let received_msg = node_channel.receive();
            match received_msg {
                Ok(message) => match message {
                    NewTransaction(tx) => {
                        let mut wallet_locked: std::sync::MutexGuard<'_, Wallet> = wallet
                            .lock()
                            .map_err(|e| NodeError::FailedToLockWallet(e.to_string()))?;

                        wallet_locked.check_tx_contains_addrs(tx, &ui_sender)?;
                    }
                    NewBlock(block_path) => {
                        let mut wallet_locked: std::sync::MutexGuard<'_, Wallet> = wallet
                            .lock()
                            .map_err(|e| NodeError::FailedToLockWallet(e.to_string()))?;
                        if !wallet_locked.has_block_been_checked(&block_path) {
                            wallet_locked.confirm_transactions(&block_path, &ui_sender)?;
                            wallet_locked.update_accounts_utxo(&block_path)?;
                        }
                    }
                    CreateNewAccount(wallet_account_info, utxo_set) => {
                        Self::create_account(&wallet, utxo_set, wallet_account_info, &ui_sender)?;
                    }
                },
                Err(e) => {
                    println!("Error receiving message in Wallet from the Node {:?}", e);
                }
            }
        }
    }

    /// Handles the communication between the wallet and the GTK UI.
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet instance in an Arc Mutex.
    /// * `ui_receiver` - The receiver channel for receiving messages from the UI.
    /// * `peer` - The peer connection to the node.
    /// * `ui_sender` - The sender channel for sending messages from the wallet to the UI.
    /// * `utxo_set_arc` - The UTXO set of the node, inside an Arc Mutex.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the function completes successfully, or an `Err` if there was an error.
    fn handle_ui_connection(
        wallet: Arc<Mutex<Wallet>>,
        ui_receiver: mpsc::Receiver<UIMessage>,
        peer: &mut TcpStream,
        wallet_node_sender: Sender<UIMessage>,
        utxo_set: Arc<Mutex<UtxoSet>>,
    ) -> Result<(), NodeError> {
        loop {
            let message = ui_receiver.recv().map_err(|_| {
                NodeError::FailedToRead("Failed to read msg from ui in wallet".to_string())
            })?;

            match message {
                UIMessage::CreateNewTransaction(target_address, amount, fee) => {
                    Self::create_and_broadcast_tx(&wallet, target_address, amount, fee, peer)?;
                }
                UIMessage::AddAccount(account_info) => {
                    Self::add_account_to_wallet(
                        account_info.clone(),
                        &wallet,
                        &utxo_set,
                        &wallet_node_sender,
                    )?;
                }
                UIMessage::AccountChanged(account_info) => {
                    Self::change_account(&wallet, account_info, &wallet_node_sender)?;
                }
                _ => {}
            }
        }
    }

    /// Changes the current account to the one specified. This is done by moving the account to the head of the
    /// accounts vector.
    /// # Arguments
    /// * `account_info` - The account info to change to.
    /// * `wallet` - The wallet instance wrapped in am arc mutex.
    /// # Returns
    /// Returns `Ok(())` if the function completes successfully, or an `Err` if there was an error.
    fn change_account(
        wallet: &Arc<Mutex<Wallet>>,
        account_info: AccountInfo,
        wallet_node_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        let mut wallet = wallet
            .lock()
            .map_err(|_| NodeError::WalletMutexError("Failed to lock wallet".to_string()))?;
        if let Some(index) = wallet.accounts.iter().position(|account| {
            account.bitcoin_address.bs58_to_string() == account_info.bitcoin_address
        }) {
            if wallet.accounts[0].bitcoin_address.bs58_to_string() == account_info.bitcoin_address {
                return Ok(());
            }
            let account = wallet.accounts.remove(index);
            wallet.accounts.insert(0, account);
            let current_account = wallet.current_account().map_err(|_| {
                NodeError::FailedToCreateTransaction("Failed to get current account".to_string())
            })?;

            wallet_node_sender
                .send(UIMessage::NewCurrentAccount(
                    current_account.copy(),
                    account_info,
                ))
                .map_err(|_| {
                    NodeError::FailedToChangeAccount(
                        "Failed to send account changed to ui".to_string(),
                    )
                })?;
        }

        Ok(())
    }

    /// Runs the wallet for the user. Creates it from the login information, and then handles
    /// the communication between the wallet and the node, and the wallet and the GTK UI.
    /// # Arguments
    /// * `utxo_set_arc` - The UTXO set for the wallet, inside an Arc Mutex to be shared between threads.
    /// * `node_channel` - The channel for communication with the node.
    /// * `ui_receiver` - The receiver channel for receiving messages from the GTK UI.
    /// * `ui_sender` - The sender channel for sending messages to the GTK UI.
    /// * `peer` - The peer to send transactions to.
    /// # Returns
    /// Returns `Ok(())` if the function completes successfully, or an `Err` if there was an error.
    pub fn run_wallet(
        utxo_set_arc: Arc<Mutex<UtxoSet>>,
        node_channel: WalletChannel,
        ui_receiver: mpsc::Receiver<UIMessage>,
        ui_sender: Sender<UIMessage>,
        peer: &mut TcpStream,
    ) -> Result<(), NodeError> {
        let wallet =
            Self::create_wallet_from_login(&ui_receiver, &utxo_set_arc, ui_sender.clone())?;

        let wallet_arc = Arc::new(Mutex::new(wallet));
        let cloned_wallet_arc = Arc::clone(&wallet_arc);
        let mut cloned_peer = peer
            .try_clone()
            .map_err(|_| NodeError::FailedToConnect("Failed to clone peer stream".to_string()))?;
        let sender = ui_sender.clone();

        thread::spawn(move || {
            Wallet::handle_ui_connection(
                cloned_wallet_arc,
                ui_receiver,
                &mut cloned_peer,
                sender,
                Arc::clone(&utxo_set_arc),
            )
            .unwrap_or_else(|e| println!("Error in wallet connection to ui: {:?}", e));
        });

        Self::handle_node_connection(Arc::clone(&wallet_arc), node_channel, ui_sender)?;

        Ok(())
    }

    /// Checks if a transaction contains user addresses and in that case adds it to the ui.
    ///
    /// # Arguments
    ///* `tx` - A `Transaction` representing the transaction to check.
    ///* ui_sender - The sender channel for sending messages from the wallet to GTK UI.
    /// # Errors
    ///
    /// Returns an `Err` variant of `NodeError` if there are any errors encountered during the process.
    fn check_tx_contains_addrs(
        &mut self,
        tx: Transaction,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        let user_addresses = self.bitcoin_addresses();
        for address in user_addresses {
            if tx.contains_address(&address) {
                self.receive_incoming_transaction(tx.clone(), &address, ui_sender)?;
            }
        }
        Ok(())
    }

    /// Creates a new wallet from the login information.
    /// # Arguments
    /// * `ui_receiver` - The receiver channel for receiving messages from the GTK UI.
    /// * `utxo_set_arc` - The UTXO set for the wallet, inside an Arc Mutex to be shared between threads.
    /// * `ui_sender` - The sender channel for sending messages from the wallet to GTK UI.
    /// # Returns
    /// Returns `Ok(())` if the function completes successfully, or an `Err` if there was an error.
    fn create_wallet_from_login(
        ui_receiver: &mpsc::Receiver<UIMessage>,
        utxo_set_arc: &Arc<Mutex<UtxoSet>>,
        ui_sender: Sender<UIMessage>,
    ) -> Result<Wallet, NodeError> {
        println!("Creating wallet...");
        let wallet = loop {
            let ui_message = ui_receiver
                .recv()
                .map_err(|_| NodeError::FailedToRead("Failed to read wallet login".to_string()))?;

            match ui_message {
                UIMessage::AddAccount(account_info) => {
                    println!("Received login message from UI");
                    let wallet =
                        Wallet::initialize_wallet_for_user(utxo_set_arc, &account_info, &ui_sender)
                            .map_err(|_| {
                                NodeError::FailedToCreateWallet(
                                    "Failed to create wallet".to_string(),
                                )
                            })?;
                    break wallet;
                }
                UIMessage::AddAccountsFromAppStart(accounts_info) => {
                    let wallet = Wallet::initialize_wallet_with_saved_accounts(
                        utxo_set_arc,
                        accounts_info,
                        &ui_sender,
                    )
                    .map_err(|_| {
                        NodeError::FailedToCreateWallet("Failed to create wallet".to_string())
                    })?;

                    break wallet;
                }
                _ => continue,
            }
        };
        Ok(wallet)
    }

    /// Creates and broadcasts a transaction, sending it to the peer and UI.
    ///
    /// # Arguments
    ///
    /// * `wallet` - An `Arc<Mutex<Wallet>>` representing the wallet.
    /// * `target_address` - The target address for the transaction.
    /// * `amount` - The amount to send in the transaction.
    /// * `fee` - The transaction fee.
    /// * `peer` - A mutable reference to the `TcpStream` for broadcasting the transaction.
    /// * `ui_sender` - A reference to the `Sender<UIMessage>` for sending UI messages.
    ///
    /// # Returns
    ///
    /// An `Ok(())` result if the transaction was created, broadcasted, and sent to the UI,
    /// or a `Result<(), NodeError>` indicating the error encountered.
    ///
    /// # Remarks
    ///
    /// This function obtains the current address from the wallet using the `obtain_current_address` method.
    /// It then locks the wallet, creates a transaction using the `create_transaction` method,
    /// broadcasts the transaction to the peer using the `broadcast_transaction` function,
    /// and sends the transaction to the UI using the `UIMessage::NewTransactionToBeSent` message.
    fn create_and_broadcast_tx(
        wallet: &Arc<Mutex<Wallet>>,
        target_address: String,
        amount: f64,
        fee: f64,
        peer: &mut TcpStream,
    ) -> Result<(), NodeError> {
        let my_address = Self::obtain_current_address(wallet)?;
        let transaction = wallet
            .lock()
            .map_err(|_| NodeError::FailedToCreateTransaction("Failed to lock wallet".to_string()))?
            .create_transaction(my_address, &target_address, amount, fee)?;
        println!(
            "Created tx: {:?} to address: {:?}",
            transaction.tx_id(),
            target_address
        );
        broadcast_transaction(transaction, peer)?;

        Ok(())
    }
    /// Adds an account to the wallet
    ///
    /// # Arguments
    ///
    /// * `wallet_node_sender` - A reference to the `Sender<UIMessage>` for sending UI messages.
    /// * `account_info` - The account information to be added to the wallet.
    /// * `wallet` - An `Arc<Mutex<Wallet>>` representing the wallet.
    /// * `utxo_set_arc` - An `Arc<Mutex<UtxoSet>>` representing the UTXO set.
    ///
    /// # Returns
    ///
    /// An `Ok(())` result if the account was added to the wallet and the UI message was sent successfully,
    /// or a `Result<(), NodeError>` indicating the error encountered.
    fn add_account_to_wallet(
        account_info: AccountInfo,
        wallet: &Arc<Mutex<Wallet>>,
        utxo_set_arc: &Arc<Mutex<UtxoSet>>,
        wallet_node_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        let mut wallet_lock = wallet
            .lock()
            .map_err(|_| NodeError::FailedToSendMessage("Failed to lock wallet".to_string()))?;
        let utxo_lock = utxo_set_arc
            .lock()
            .map_err(|_| NodeError::FailedToSendMessage("Failed to lock utxo set".to_string()))?;
        wallet_lock.add_account(&utxo_lock, account_info, wallet_node_sender)?;
        Ok(())
    }

    ///Returns the first account's address as a string.
    /// That account is the current account.
    /// # Arguments
    /// * `wallet` - The wallet to get the address from wrapped in an ArcMutex.
    /// # Returns
    /// Returns the first account's address as a string or a NodeError if there was an error.
    fn obtain_current_address(wallet: &Arc<Mutex<Wallet>>) -> Result<String, NodeError> {
        let my_address = wallet
            .lock()
            .map_err(|_| NodeError::FailedToCreateTransaction("Failed to lock wallet".to_string()))?
            .current_account()?
            .bitcoin_address
            .bs58_to_string();

        Ok(my_address)
    }

    /// Returns the first account in the wallet, this is the current account.
    pub fn current_account(&self) -> Result<&Account, NodeError> {
        self.accounts
            .first()
            .ok_or(NodeError::FailedToObtainAccount(
                "No account found".to_string(),
            ))
    }
    /// Creates a new account and adds it to the wallet.
    ///
    /// # Arguments
    ///
    /// * `wallet` - An `Arc<Mutex<Wallet>>` representing the wallet.
    /// * `utxo_set` - The UTXO (Unspent Transaction Output) set.
    /// * `wallet_account_info` - The information of the new account to be added to the wallet.
    /// * `wallet_node_sender` - A reference to the `Sender<UIMessage>` for sending UI messages.
    ///
    /// # Returns
    ///
    /// An `Ok(())` result if the account was created and added successfully, or a `Result<(), NodeError>`
    /// indicating the error encountered.
    ///
    /// # Remarks
    ///
    /// This function prints a log message indicating the addition of a new account to the wallet.
    /// It locks the `wallet` and calls the `add_account` method on the locked wallet,
    /// passing the `utxo_set`, extracted Bitcoin address, and extracted private key from `wallet_account_info` as arguments.
    fn create_account(
        wallet: &Arc<Mutex<Wallet>>,
        utxo_set: UtxoSet,
        wallet_account_info: AccountInfo,
        wallet_node_sender: &Sender<UIMessage>,
    ) -> Result<(), NodeError> {
        println!("Adding a new account to the wallet");
        let mut wallet_locked = wallet
            .lock()
            .map_err(|_| NodeError::WalletMutexError("Failed to lock wallet".to_string()))?;
        wallet_locked.add_account(&utxo_set, wallet_account_info, wallet_node_sender)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use glib::Receiver;

    use crate::{
        block::retrieve_transactions_from_block,
        transactions::{tx_input::TxInput, tx_output::TxOutput},
    };

    use super::*;

    #[test]
    fn test_remove_account() {
        let mut utxo_set = UtxoSet::new();

        utxo_set
            .update(
                &"blocks-test/0000000000000014e9428b9aa7427ec63e867030c1d77afeb1b182537e15be0a.bin"
                    .to_string(),
            )
            .unwrap();
        let wallet_info = AccountInfo::new_from_values(
            "mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string(),
            "a".to_string(),
            "a".to_string(),
        );
        let (wallet_node_sender, wallet_node_receiver): (Sender<UIMessage>, Receiver<UIMessage>) =
            glib::MainContext::channel(glib::Priority::default());
        let mut wallet = Wallet::initialize_wallet_for_user(
            &Arc::new(Mutex::new(utxo_set.clone())),
            &wallet_info,
            &wallet_node_sender,
        )
        .unwrap();

        let new_account = AccountInfo::new_from_values(
            "mtEoVpBV5H8bbmNDEPwaoJHXnF1MxbkkQf".to_string(),
            "a".to_string(),
            "a".to_string(),
        );
        wallet
            .add_account(&utxo_set, new_account, &wallet_node_sender)
            .unwrap();

        assert!(wallet
            .remove_account(&"mtEoVpBV5H8bbmNDEPwaoJHXnF1MxbkkQf".to_string())
            .is_ok());
        assert!(wallet.accounts.len() == 1);

        wallet_node_receiver.attach(None, move |_| glib::Continue(true));
    }

    #[test]
    fn test_receive_tx() -> Result<(), NodeError> {
        let mut utxo_set = UtxoSet::new();
        let block_path =
            "blocks-test/000000000000000a2b6d192ab83f7706e60cece100aabb45a4b9ce4656b6a702.bin";

        utxo_set.update(&block_path.to_string())?;
        let wallet_info = AccountInfo::new_from_values(
            "mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string(),
            "a".to_string(),
            "a".to_string(),
        );
        let (wallet_node_sender, wallet_node_receiver): (Sender<UIMessage>, Receiver<UIMessage>) =
            glib::MainContext::channel(glib::Priority::default());
        let mut wallet = Wallet::initialize_wallet_for_user(
            &Arc::new(Mutex::new(utxo_set)),
            &wallet_info,
            &wallet_node_sender,
        )
        .unwrap();

        let tx_output = vec![TxOutput {
            value: 1,
            pk_script: vec![0, 1, 2],
            pk_script_bytes: crate::compact_size::CompactSize::U16(3),
            tx_id: vec![4],
            index: 0,
            block_path: block_path.to_string(),
        }];
        let tx_input = vec![TxInput {
            previous_output: crate::transactions::outpoint::Outpoint {
                tx_id: vec![4],
                index: 0,
            },
            script_bytes: crate::compact_size::CompactSize::U16(8),
            signature_script: vec![01],
            sequence: 0,
        }];
        let tx = Transaction {
            version: 4,
            tx_in_count: crate::compact_size::CompactSize::U16(8),
            tx_inputs: tx_input,
            tx_out_count: crate::compact_size::CompactSize::U16(1),
            tx_outputs: tx_output,
            lock_time: 0,
        };
        let bc_address =
            BitcoinAddress::from_string(&"mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string()).unwrap();
        wallet.receive_incoming_transaction(tx.clone(), &bc_address, &wallet_node_sender)?;
        assert!(wallet.accounts[0].unconfirmed_transactions.received.len() == 1);
        assert_eq!(
            wallet.accounts[0].unconfirmed_transactions.received[0].tx_id(),
            tx.tx_id()
        );
        wallet_node_receiver.attach(None, move |_| glib::Continue(true));
        Ok(())
    }

    #[test]
    fn test_confirm_tx() -> Result<(), NodeError> {
        let block_path =
            "blocks-test/000000000000000a2b6d192ab83f7706e60cece100aabb45a4b9ce4656b6a702.bin"
                .to_string();
        let tx_unconfirmed = retrieve_transactions_from_block(&block_path)
            .unwrap()
            .first()
            .unwrap()
            .clone();
        let mut utxo_set = UtxoSet::new();
        utxo_set.update(&block_path)?;
        let wallet_info = AccountInfo::new_from_values(
            "mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string(),
            "a".to_string(),
            "a".to_string(),
        );
        let (wallet_node_sender, wallet_node_receiver): (Sender<UIMessage>, Receiver<UIMessage>) =
            glib::MainContext::channel(glib::Priority::default());
        let mut wallet = Wallet::initialize_wallet_for_user(
            &Arc::new(Mutex::new(utxo_set)),
            &wallet_info,
            &wallet_node_sender,
        )
        .unwrap();

        assert_eq!(
            wallet.accounts[0].unconfirmed_transactions.received.len(),
            0
        );
        assert_eq!(wallet.accounts[0].confirmed_transactions.received.len(), 0);

        wallet.accounts[0]
            .unconfirmed_transactions
            .received
            .push(tx_unconfirmed.clone());
        wallet.confirm_transactions(&block_path, &wallet_node_sender)?;
        assert!(wallet.accounts[0].unconfirmed_transactions.received.len() == 0);
        assert!(wallet.accounts[0].confirmed_transactions.received.len() == 1);
        assert_eq!(
            wallet.accounts[0].confirmed_transactions.received[0].tx_id(),
            tx_unconfirmed.tx_id()
        );
        wallet_node_receiver.attach(None, move |msg| {
            match msg {
                UIMessage::NewTransactionsConfirmed(transacion_data) => {
                    let tx_id = transacion_data.txs.received.first().unwrap().tx_id();
                    assert_eq!(tx_id, tx_unconfirmed.to_bytes());
                }
                _ => {}
            }
            glib::Continue(true)
        });

        Ok(())
    }
}
