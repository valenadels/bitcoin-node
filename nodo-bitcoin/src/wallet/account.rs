use super::{
    bitcoin_address::BitcoinAddress, transactions_spent_received::TransactionsSpentAndReceived,
};
use crate::{
    block::retrieve_transactions_from_block,
    compact_size::CompactSize,
    constants::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160, PK_HASH_LENGTH},
    node_error::NodeError,
    transactions::{
        pk_script::PkScript, signature_script::SignatureScript, transaction::Transaction,
        tx_input::TxInput, tx_output::TxOutput, utxo_set::UtxoSet,
    },
    ui::{components::transactions_confirmed_data::Amount, ui_message::UIMessage},
};
use bitcoin_hashes::sha256;
use glib::Sender;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

type Wif = String;

#[derive(Debug, Clone)]
/// Represents an Account for the user.
pub struct Account {
    /// The address of the account.
    pub bitcoin_address: BitcoinAddress,
    /// The private key in WIF format.
    pub private_key: Wif,
    /// The UTXO set for the account.
    pub utxo_set: UtxoSet,

    pub unconfirmed_transactions: TransactionsSpentAndReceived,

    pub confirmed_transactions: TransactionsSpentAndReceived,
}

impl Account {
    /// Returns an account for the user.
    pub fn new(
        utxo_set: &UtxoSet,
        bitcoin_address_string: String,
        private_key: String,
    ) -> Result<Account, NodeError> {
        let bitcoin_address = BitcoinAddress::from_string(&bitcoin_address_string)?;
        let users_pk_hash = BitcoinAddress::to_pk_hash(&bitcoin_address);

        let users_utxo_set = utxo_set.users_utxo_set(&users_pk_hash);

        let account = Account {
            bitcoin_address,
            private_key,
            utxo_set: users_utxo_set,
            unconfirmed_transactions: TransactionsSpentAndReceived::new(),
            confirmed_transactions: TransactionsSpentAndReceived::new(),
        };

        Ok(account)
    }

    /// Returns the addresses BitcoinAddress.
    pub fn bitcoin_address(&self) -> BitcoinAddress {
        self.bitcoin_address.clone()
    }

    /// Gets the public key hash from a public key script.
    pub fn pk_script_to_pk_hash(pk_script: &PkScript) -> Result<Vec<u8>, NodeError> {
        if pk_script.is_empty() {
            return Err(NodeError::NotP2PKHScript(
                "This Public Key Script is empty".to_string(),
            ));
        }

        let mut pk_hash = pk_script.to_owned();

        if pk_hash[0] == OP_DUP {
            pk_hash.remove(0);
        } else {
            return Err(NodeError::NotP2PKHScript(
                "This Public Key Script is not a P2PKH script".to_string(),
            ));
        }

        if pk_hash[0] == OP_HASH160 {
            pk_hash.remove(0);
        } else {
            return Err(NodeError::NotP2PKHScript(
                "This Public Key Script is not a P2PKH script".to_string(),
            ));
        }

        pk_hash.remove(0);
        pk_hash.remove(pk_hash.len() - 1);
        pk_hash.remove(pk_hash.len() - 1);

        Ok(pk_hash)
    }

    /// Returns the pk script for the given public key hash.
    pub fn pk_hash_to_pk_script(pk_hash: &Vec<u8>) -> PkScript {
        let mut pk_script = vec![OP_DUP, OP_HASH160, PK_HASH_LENGTH];
        pk_script.extend(pk_hash);
        pk_script.extend(vec![OP_EQUALVERIFY, OP_CHECKSIG]);

        pk_script
    }

    /// The private key is given in a Wallet Import Format (WIF) string. This function parses the WIF and returns the private key that is contained in it, in bytes.
    fn wif_to_private_key(&self) -> Result<Vec<u8>, NodeError> {
        let private_key_decoded = bs58::decode(&self.private_key).into_vec().map_err(|_| {
            NodeError::SigningError("Failed to decode private key from Base58".to_string())
        })?;

        let pk = private_key_decoded[1..private_key_decoded.len() - 4].to_vec();

        Ok(pk)
    }

    /// Returns the balance for the given Bitcoin address in the UTXO set.
    pub fn calculate_balance(users_pk_hash: &Vec<u8>, utxo_set: &UtxoSet) -> f64 {
        let mut balance = 0.0;

        for tx_tuple in utxo_set.set.iter() {
            let tx_outputs = tx_tuple.1;

            for tx_output in tx_outputs {
                let tx_output_pk_hash = match Self::pk_script_to_pk_hash(&tx_output.pk_script) {
                    Ok(pk_hash) => pk_hash,
                    Err(_) => {
                        println!("This is not a P2PKH script");
                        continue;
                    }
                };
                if users_pk_hash == &tx_output_pk_hash {
                    balance += tx_output.value();
                }
            }
        }
        balance
    }

    /// Returns the balance for the user.
    pub fn balance_for_user(&self) -> f64 {
        Self::calculate_balance(
            &BitcoinAddress::to_pk_hash(&self.bitcoin_address),
            &self.utxo_set,
        )
    }

    /// Creates a list of unsigned transaction inputs (TxInput) to spend UTXOs from the current wallet.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of coins (in f64) that should be spent in the transaction.
    ///
    /// # Returns
    ///
    /// A tuple containing the following elements:
    /// * `txs_inputs` - A vector of TxInput structs representing the unsigned transaction inputs.
    /// * `total_amount` - The total amount of coins (in f64) that will be spent from the UTXOs.
    /// * `pk_scripts` - A vector of PkScript structs representing the public key scripts associated with the UTXOs to spend.
    ///
    /// # Errors
    ///
    /// Returns a NodeError if there are any issues with searching for UTXOs or creating the transaction inputs.
    pub fn create_unsigned_inputs(
        &self,
        amount: &f64,
    ) -> Result<(Vec<TxInput>, f64, Vec<PkScript>), NodeError> {
        let tx_outs_to_spend = self.utxo_set.search_utxos_to_spend(amount)?;
        let mut txs_inputs = Vec::new();

        for tx_out_to_spend in tx_outs_to_spend.iter() {
            let tx_in =
                TxInput::new_unsigned(&tx_out_to_spend.tx_id, &(tx_out_to_spend.index as u32), &[]);

            txs_inputs.push(tx_in);
        }

        Ok((
            txs_inputs,
            UtxoSet::sum_of_outs(&tx_outs_to_spend),
            TxOutput::pk_scripts(&tx_outs_to_spend),
        ))
    }

    /// Creates an unsigned transaction.
    ///
    /// This function constructs an unsigned transaction by searching for unspent transaction outputs (UTXOs)
    /// to spend and creating transaction inputs and outputs accordingly. The resulting transaction is not signed.
    ///
    /// # Arguments
    ///
    /// * `target_address_str` - The target Bitcoin address as a string.
    /// * `amount` - The amount of Bitcoin to transfer.
    /// * `fee` - The fee to pay for the transaction.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure. If successful, `Ok()` is returned, containing the unsigned transaction and a vec of PkScripts to be used to sign the transaction.
    pub fn create_unsigned_transaction(
        &self,
        target_address_str: &String,
        amount: f64,
        fee: f64,
    ) -> Result<(Transaction, Vec<PkScript>), NodeError> {
        if self.balance_for_user() < amount {
            return Err(NodeError::NotEnoughCoins(
                "Not enough coins to spend".to_string(),
            ));
        }

        let (txs_inputs, value_spent, pk_scripts) = self.create_unsigned_inputs(&amount)?;

        let change = value_spent - amount;
        let change_script = BitcoinAddress::to_pk_script(&self.bitcoin_address);
        let change_tx_out = TxOutput::new(change, change_script, 0);

        let target_address = BitcoinAddress::from_string(target_address_str)?;
        let target_script = BitcoinAddress::to_pk_script(&target_address);
        let target_tx_out = TxOutput::new(amount - fee, target_script, 1);

        let transaction = Transaction::new_unsigned(txs_inputs, vec![change_tx_out, target_tx_out]);

        Ok((transaction, pk_scripts))
    }

    /// Creates a list of signature scripts for the given transaction's inputs.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A mutable reference to the transaction for which script signatures need to be created.
    /// * `pk_scripts` - A vector containing the public key scripts (PkScript) associated with the transaction's inputs.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of signature scripts for each transaction input.
    ///
    /// # Errors
    ///
    /// Returns a NodeError if there are any issues with signing the transaction or parsing the private key
    fn create_script_sigs(
        &self,
        transaction: &mut Transaction,
        pk_scripts: Vec<PkScript>,
    ) -> Result<Vec<SignatureScript>, NodeError> {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&self.wif_to_private_key()?).map_err(|_| {
            NodeError::SigningError("Failed to parse private key into secret key".to_string())
        })?;

        let mut script_sigs = Vec::new();

        for (i, pk_script) in pk_scripts
            .iter()
            .enumerate()
            .take(transaction.tx_inputs.len())
        {
            let message = Message::from_hashed_data::<sha256::Hash>(
                &transaction.individual_signature_hash(i, pk_script.clone()),
            );

            let mut signature_bytes = secp
                .sign_ecdsa(&message, &secret_key)
                .serialize_der()
                .to_vec();
            signature_bytes.push(1_u8);

            let sec_public_key = PublicKey::from_secret_key(&secp, &secret_key)
                .serialize()
                .to_vec();

            let mut script_sig = CompactSize::new(signature_bytes.len()).to_bytes();
            script_sig.extend(signature_bytes);
            script_sig.extend(CompactSize::new(sec_public_key.len()).to_bytes());
            script_sig.extend(sec_public_key);

            script_sigs.push(script_sig);
        }

        Ok(script_sigs)
    }

    /// Sign the given transaction with the provided private key.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to sign.
    /// * `private_key` - The private key used for signing.
    /// * `pk_scripts` - A vector containing the public key scripts (PkScript) associated with the transaction's inputs.
    ///
    /// # Returns
    ///
    /// A `Result` containing the signed transaction.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is an issue with signing the transaction.
    pub fn sign_transaction(
        &self,
        transaction: &mut Transaction,
        pk_scripts: Vec<PkScript>,
    ) -> Result<(), NodeError> {
        let script_sigs = self.create_script_sigs(transaction, pk_scripts)?;
        transaction.add_script_sigs(script_sigs);
        transaction.add_tx_id_to_tx_outs();

        Ok(())
    }

    /// Creates a valid transaction, ready to be broadcasted to the bitcoin testnet.
    ///
    /// # Arguments
    ///
    /// * `target_address_str` - A reference to a string containing the target address to send the coins.
    /// * `amount` - The amount of coins to send to the target address.
    /// * `fee` - The transaction fee to be included in the transaction.
    ///
    /// # Returns
    ///
    /// A Result containing the transaction if successful.
    ///
    /// # Errors
    ///
    /// Returns a NodeError if there are any issues with creating the unsigned transaction or signing the transaction.
    pub fn create_transaction(
        &self,
        target_address_str: &String,
        amount: f64,
        fee: f64,
    ) -> Result<Transaction, NodeError> {
        let (mut transaction, pk_scripts) =
            self.create_unsigned_transaction(target_address_str, amount, fee)?;
        self.sign_transaction(&mut transaction, pk_scripts)?;

        Ok(transaction)
    }

    /// Updates the UTXO set for this account.
    pub fn update_utxo(&mut self, block_path: &String) -> Result<(), NodeError> {
        self.utxo_set
            .update_for_account(block_path, &self.bitcoin_address)
    }

    /// Implements the Copy trait for Account.
    pub fn copy(&self) -> Account {
        Account {
            bitcoin_address: self.bitcoin_address.clone(),
            private_key: self.private_key.clone(),
            utxo_set: self.utxo_set.clone(),
            unconfirmed_transactions: self.unconfirmed_transactions.clone(),
            confirmed_transactions: self.confirmed_transactions.clone(),
        }
    }

    /// Adds a new unconfirmed transaction to the account.
    /// # Arguments
    /// * `transaction` - The transaction to add.
    pub fn add_new_unconfirmed_transaction(&mut self, transaction: Transaction) {
        for tx_input in transaction.tx_inputs.iter() {
            if self.utxo_set.contains_key(&tx_input.previous_output.tx_id) {
                self.unconfirmed_transactions.add_spent(transaction);
                return;
            }
        }
        self.unconfirmed_transactions.add_received(transaction);
    }

    /// Confirms transactions that where previously unconfirmed, because they appeared in a new block, updating the Node's state and notifying the UI.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to the Node object.
    /// * `path` - The path to the block from which transactions will be confirmed.
    /// * `ui_sender` - The sender channel to communicate with the UI.
    ///
    /// # Errors
    ///
    /// This function can return a `NodeError` under the following circumstances:
    ///
    /// * If retrieving transactions from the block fails.
    /// * If sending a message to the UI fails.
    pub fn confirm_transactions(
        &mut self,
        path: &String,
        ui_sender: &Sender<UIMessage>,
    ) -> Result<TransactionsSpentAndReceived, NodeError> {
        let transactions = retrieve_transactions_from_block(path)?;
        let mut confirmed_tx_to_ui = TransactionsSpentAndReceived::new();

        self.update_transactions_if_confirmed(transactions, &mut confirmed_tx_to_ui);

        confirmed_tx_to_ui.send_confirmations_to_ui(ui_sender)?;

        Ok(confirmed_tx_to_ui)
    }

    /// Updates the vector of unconfirmed transactions, removing those that have been confirmed.
    fn update_transactions_if_confirmed(
        &mut self,
        transactions: Vec<Transaction>,
        confirmed_tx_to_ui: &mut TransactionsSpentAndReceived,
    ) {
        for transaction in transactions {
            if self.unconfirmed_transactions.remove_spent(&transaction) {
                self.confirmed_transactions.add_spent(transaction.clone());
                confirmed_tx_to_ui.add_spent(transaction);
            } else if self.unconfirmed_transactions.remove_received(&transaction) {
                self.confirmed_transactions
                    .add_received(transaction.clone());
                confirmed_tx_to_ui.add_received(transaction);
            };
        }
    }

    /// # Returns
    /// The amount of coins that have been spent but not confirmed yet.
    pub fn unconfirmed_spent_balance(&mut self) -> Amount {
        let mut account = self.copy();
        self.unconfirmed_transactions.spent_balance(&mut account)
    }

    /// # Returns
    /// The amount of coins that have been received but not confirmed yet.
    pub fn unconfirmed_received_balance(&self) -> Amount {
        let users_pk_hash = &BitcoinAddress::to_pk_hash(&self.bitcoin_address);
        self.unconfirmed_transactions
            .received_balance(users_pk_hash)
    }
}

#[cfg(test)]
mod test {
    use crate::{node_error::NodeError, utils::Utils};

    use super::*;

    #[test]
    fn test_get_balance_for_user_with_no_transactions() -> Result<(), NodeError> {
        let utxo_set = UtxoSet::new();

        let bitcoin_address =
            BitcoinAddress::from_string(&String::from("mr1J99hL9xgGu7T5XHR4Y85DwUkuwLMmMQ"))?;

        let users_pk_hash = BitcoinAddress::to_pk_hash(&bitcoin_address);

        let balance = Account::calculate_balance(&users_pk_hash, &utxo_set);

        assert_eq!(balance, 0.0);

        Ok(())
    }

    #[test]
    fn test_get_balance_for_user_with_transactions() -> Result<(), NodeError> {
        let mut utxo_set = UtxoSet::new();

        utxo_set.update(
            &"blocks-test/0000000000000014e9428b9aa7427ec63e867030c1d77afeb1b182537e15be0a.bin"
                .to_string(),
        )?;

        let bitcoin_address =
            BitcoinAddress::from_string(&String::from("mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk"))?;

        let users_pk_hash = BitcoinAddress::to_pk_hash(&bitcoin_address);

        let balance = Account::calculate_balance(&users_pk_hash, &utxo_set);

        assert_eq!(balance, 0.02432823);

        Ok(())
    }

    #[test]
    fn test_create_transaction1() {
        let mut utxo_set = UtxoSet::new();
        utxo_set
            .update(
                &"blocks-test/0000000000000005847b65f037ec3d08f499c3c22ae6723ffefee1adca3e9af5.bin"
                    .to_string(),
            )
            .unwrap();
        let account = Account::new(
            &utxo_set,
            String::from("mna7LXQEht1uRaUEKv1UGvF8N1eqMXCATC"),
            String::from("92GMMJkoBsXuzFNod6a8fgPFworara3HS6zgGHTFR1Xfo1c9Je5"),
        )
        .unwrap();

        let (mut tx, pk_scripts) = account
            .create_unsigned_transaction(
                &String::from("mv4rnyY3Su5gjcDNzbMLKBQkBicCtHUtFB"),
                0.01,
                0.005,
            )
            .unwrap();

        account.sign_transaction(&mut tx, pk_scripts).unwrap();

        println!(
            "SCRIPT SIG: {:?}",
            Utils::bytes_to_hex(&tx.tx_inputs[0].signature_script)
        );
        println!(
            "SIGNED TRANSACTION BYTES TO HEX: {:?}",
            Utils::bytes_to_hex(&tx.to_bytes())
        );
    }
    #[test]
    fn test_create_transaction2() {
        let mut utxo_set = UtxoSet::new();
        utxo_set
            .update(
                &"blocks-test/000000000000001c49d310478ff08742c26efb8f24d8756412996c51ed384a67.bin"
                    .to_string(),
            )
            .unwrap();
        let account = Account::new(
            &utxo_set,
            String::from("mmKLrA7dvdtGez1GH9ChBkQ6FLUiNr3mFz"),
            String::from("9319Nrhiz9UD4EgeW3n18YpRTbcjTYkvS57b3WxX96P24bGFxHv"),
        )
        .unwrap();

        let (mut tx, pk_scripts) = account
            .create_unsigned_transaction(
                &String::from("mv4rnyY3Su5gjcDNzbMLKBQkBicCtHUtFB"),
                0.01,
                0.005,
            )
            .unwrap();

        account.sign_transaction(&mut tx, pk_scripts).unwrap();

        println!(
            "SCRIPT SIG: {:?}",
            Utils::bytes_to_hex(&tx.tx_inputs[0].signature_script)
        );
        println!(
            "SIGNED TRANSACTION BYTES TO HEX: {:?}",
            Utils::bytes_to_hex(&tx.to_bytes())
        );
    }

    #[test]
    fn test_create_transaction_spends_two_outputs() {
        let mut utxo_set = UtxoSet::new();
        utxo_set
            .update(
                &"blocks-test/000000000000001f621da3e094a50ba0842a21694d161345581347ff0ec67a93.bin"
                    .to_string(),
            )
            .unwrap();

        utxo_set
            .update(
                &"blocks-test/0000000000001fdc30a4b54fff00ae2494add9f41297b1cc426d8b8230129a38.bin"
                    .to_string(),
            )
            .unwrap();

        let account = Account::new(
            &utxo_set,
            String::from("mna7LXQEht1uRaUEKv1UGvF8N1eqMXCATC"),
            String::from("92GMMJkoBsXuzFNod6a8fgPFworara3HS6zgGHTFR1Xfo1c9Je5"),
        )
        .unwrap();

        println!("BALANCE: {:?}", account.balance_for_user());
        let (mut tx, pk_scripts) = account
            .create_unsigned_transaction(
                &String::from("mmKLrA7dvdtGez1GH9ChBkQ6FLUiNr3mFz"),
                0.05,
                0.002,
            )
            .unwrap();

        account.sign_transaction(&mut tx, pk_scripts).unwrap();

        println!(
            "SIGNED TRANSACTION BYTES TO HEX: {:?}",
            Utils::bytes_to_hex(&tx.to_bytes())
        );
    }

    #[test]
    fn test_create_transaction_spends_more_outputs() {
        let mut utxo_set = UtxoSet::new();
        utxo_set
            .update(
                &"blocks-test/000000000000001fe07dd7d936489026a1dc1906ba797f0cac12b645367c9952.bin"
                    .to_string(),
            )
            .unwrap();

        utxo_set
            .update(
                &"blocks-test/0000000000000008771c98eab6cbcea3c63138d3715e67d244b52dd183053f80.bin"
                    .to_string(),
            )
            .unwrap();

        utxo_set
            .update(
                &"blocks-test/000000000000000c5d6cc58f545057a781c46c100a0f2ea5f8f6a31c1b44c784.bin"
                    .to_string(),
            )
            .unwrap();

        utxo_set
            .update(
                &"blocks-test/000000000000001ea1833f96dbbe35fd5e0d0f2d6fce810bd91a3d236163dc94.bin"
                    .to_string(),
            )
            .unwrap();

        utxo_set
            .update(
                &"blocks-test/00000000000026d0538e1c26d2362bb6078efd9609bb1954117c6e1aa81811bc.bin"
                    .to_string(),
            )
            .unwrap();

        let account = Account::new(
            &utxo_set,
            String::from("mmKLrA7dvdtGez1GH9ChBkQ6FLUiNr3mFz"),
            String::from("9319Nrhiz9UD4EgeW3n18YpRTbcjTYkvS57b3WxX96P24bGFxHv"),
        )
        .unwrap();

        println!("BALANCE: {:?}", account.balance_for_user());
        let (mut tx, pk_scripts) = account
            .create_unsigned_transaction(
                &String::from("mna7LXQEht1uRaUEKv1UGvF8N1eqMXCATC"),
                0.0065,
                0.0002,
            )
            .unwrap();

        account.sign_transaction(&mut tx, pk_scripts).unwrap();

        println!(
            "SIGNED TRANSACTION BYTES TO HEX: {:?}",
            Utils::bytes_to_hex(&tx.to_bytes())
        );
    }
}
