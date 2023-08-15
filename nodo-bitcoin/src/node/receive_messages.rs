use std::net::TcpStream;

use crate::{
    connectors::peer_connector::{receive_message, send_message},
    constants::{COMMAND_NAME_PONG, MSG_BLOCK, MSG_TX},
    header::Header,
    messages::{get_data_message::GetDataMessage, inv_message::InvMessage},
    node_error::NodeError,
    transactions::transaction::Transaction,
};

/// Handles a `ping` message received from a peer. The function receives a `ping` message from a peer over the given `TcpStream` and returns a `pong` message to the peer.
/// The `ping` message is expected to return via the TcpStream a 8-byte nonce. The function reads the message from the stream and returns a `pong` message containing the nonce.
///
/// # Arguments
///
/// * `recv_header` - A vector of bytes representing the received message header.
/// * `stream` - A mutable reference to a `TcpStream` representing the connection to the peer that is expected to send the `ping` message.
///
/// # Returns
///
/// A `Result` indicating whether the message was successfully handled (with an empty Ok()), or whether an error occurred.
///
/// # Errors
///
/// Returns a `NodeError` if an error occurs while receiving the message over the TCP connection.
pub fn send_pong_message(stream: &mut TcpStream, header: &Header) -> Result<(), NodeError> {
    let payload_size = header.payload_size();
    let ping_payload = receive_message(stream, payload_size)?;
    let pong_header = Header::create_header(&ping_payload, COMMAND_NAME_PONG)?;
    let mut pong_message = Vec::new();
    pong_message.extend_from_slice(&pong_header);
    pong_message.extend_from_slice(&ping_payload);
    if let Err(e) = send_message(stream, pong_message) {
        Err(e)
    } else {
        Ok(())
    }
}

/// Handles an incoming 'addr' message received from its peer. The addr message relays connection information for peers on the network. Each peer which wants to accept incoming connections creates an “addr” or “addrv2” message providing its connection information and then sends that message to its peers unsolicited.
/// Some of the peers can choose to send that information to their peers (also unsolicited), some of which further distribute it, allowing decentralized peer discovery for any program already on the network. The peers can also choose to read the addr but ignore it.
///
/// # Arguments
///
/// * `recv_header` - A slice containing the message header received.
/// * `stream` - A mutable reference to a TcpStream connected to a Bitcoin peer.
///
/// # Returns
///
/// This function returns a `Result` indicating whether the operation was successful or not.
///
/// # Errors
///
/// This function may return a `NodeError` if there was an error reading or parsing the message.
pub fn receive_addr_message(stream: &mut TcpStream, header: &Header) -> Result<(), NodeError> {
    let payload_size = header.payload_size();
    receive_message(stream, payload_size)?;

    Ok(())
}

/// Handles an incoming 'feefilter' message received from its peer. The “feefilter” messages allows a node to inform its peers that it will not accept transactions below a specified fee rate into its mempool, and therefore that the peers can skip relaying inv messages for transactions below that fee rate to that node. The receiving peer may choose to ignore the message and not filter transaction inv messages, which is what is done in our case.
///
/// # Arguments
///
/// * `recv_header` - A slice containing the message header received.
/// * `stream` - A mutable reference to a TcpStream connected to a Bitcoin peer.
///
/// # Returns
///
/// This function returns a `Result` indicating whether the operation was successful or not.
///
/// # Errors
///
/// This function may return a `NodeError` if there was an error reading or parsing the message.
pub fn receive_feefilter_message(stream: &mut TcpStream, header: &Header) -> Result<(), NodeError> {
    let payload_size = header.payload_size();
    receive_message(stream, payload_size)?;
    Ok(())
}

// Handles an "inv" message received from a peer by ignoring it. The "inv" message
/// contains a list of inventory vectors that represent objects such as blocks,
/// transactions, or other data that the peer has. This function ignores the
/// inventory vectors and does not request any of the objects from the peer.
///
/// # Arguments
///
/// * `recv_header` - A slice of bytes representing the message header.
/// * `stream` - A mutable reference to a `TcpStream` representing the network
///              connection to the peer.
///
/// # Returns
///
/// * `Ok(())` if the "inv" message was handled successfully.
/// * `Err(NodeError)` if an error occurred while handling the "inv" message.
pub fn receive_inv_message(stream: &mut TcpStream, header: &Header) -> Result<(), NodeError> {
    let inv_message = receive_message(stream, header.payload_size())?;
    InvMessage::from_bytes(&inv_message)?;

    Ok(())
}

///Handles a "not found" message received from a peer by ignoring it. The "not found" message is sent in response to a "get data" message if any of the requested data objects could not be retrieved.
pub fn receive_not_found_message(stream: &mut TcpStream, header: &Header) -> NodeError {
    receive_message(stream, header.payload_size()).err();
    NodeError::SyncNodeDoesNotHaveTheBlock("NotFound".to_string())
}

/// Handles the inv message received over a TCP stream, if the inv type is MSG_BLOCK, it extracts the block hash, if the inv type is MSG_TX, it sends a GetData Message to receive the new transaction.
///
/// # Arguments
///
/// * `header` - A byte slice representing the header of the inv message.
/// * `stream` - A mutable reference to a `TcpStream` to receive the inv message payload from.
///
/// # Returns
///
/// Returns a `Vec<u8>` representing the block hash if it is found in the inv message,
/// or an empty vector if no block hash is found.
///
/// # Errors
///
/// Returns a `NodeError` if there was an error while receiving the inv message or converting it to `InvMessage`.
pub fn receive_and_handle_inv_message(
    stream: &mut TcpStream,
    header: &Header,
) -> Result<Vec<u8>, NodeError> {
    let inv_message = receive_message(stream, header.payload_size())?;
    let inv_message = InvMessage::from_bytes(&inv_message)?;

    for inv in inv_message.inventory {
        if inv.inv_type == MSG_BLOCK {
            return Ok(inv.hash.to_vec());
        } else if inv.inv_type == MSG_TX {
            let data_message = GetDataMessage::new(1, MSG_TX, inv.hash)?;
            data_message.send_message(stream)?;
        }
    }
    Ok(Vec::new())
}

/// Receives a transaction message over a TCP Stream
///
/// # Arguments
///
/// * `stream` - A mutable reference to a TCP stream.
/// * `wallet` - An `Arc<Mutex<Wallet>>` representing the wallet shared across threads.
///
/// # Returns
///
/// An empty `Result` indicating success or an error.
///
/// # Errors
///
/// This function may return an error if there is an issue reading the transaction from the stream,
/// acquiring a lock on the wallet mutex, or sending user addresses to the node. The specific error
/// types are defined in the `NodeError` enum.
pub fn receive_tx_message(stream: &mut TcpStream) -> Result<Transaction, NodeError> {
    Transaction::read_transaction(stream)
}
