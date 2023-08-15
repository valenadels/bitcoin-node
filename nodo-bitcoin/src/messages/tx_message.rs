use std::net::TcpStream;

use crate::{
    connectors::peer_connector::send_message, constants::COMMAND_NAME_TX, header::Header,
    node_error::NodeError, transactions::transaction::Transaction,
};

/// The `TxMessage` struct represents a Bitcoin `tx` message.
pub struct TxMessage;

impl TxMessage {
    /// Receives a `tx` message from a peer over the given `TcpStream`.
    pub fn receive_tx_message(stream: &mut TcpStream) -> Result<Transaction, NodeError> {
        let _tx_message_header = Header::new(stream)?;
        Transaction::read_transaction(stream)
    }
    /// Sends a `tx` message to a peer over the given `TcpStream`.
    pub fn send_tx_message(
        transaction: &Transaction,
        stream: &mut TcpStream,
    ) -> Result<(), NodeError> {
        let tx_message_bytes = transaction.to_bytes();
        let header_tx = Header::create_header(&tx_message_bytes, COMMAND_NAME_TX)?;

        let mut bytes = vec![];
        let mut tx_id = transaction.tx_id();
        tx_id.reverse();

        bytes.extend(header_tx);
        bytes.extend(tx_message_bytes);
        send_message(stream, bytes)?;

        println!("Broadcasted tx: {:?}", tx_id);
        Ok(())
    }
}
