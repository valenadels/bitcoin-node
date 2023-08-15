use std::net::TcpStream;

use crate::{
    connectors::peer_connector::{receive_message, send_message},
    constants::COMMAND_NAME_GET_DATA,
    header::Header,
    node_error::NodeError,
};

use super::inv_message::InvMessage;

/// GetDataMessage is a message that requests data from a peer. (Blocks in this case)
/// Its structure is identical to an Inv message, that is why it is an alias.
pub type GetDataMessage = InvMessage;

impl GetDataMessage {
    /// Sends a "getdata" message with the specified block hash to the given TCP stream.
    ///
    /// This function creates and sends a "getdata" message with the specified block hash to the given
    /// TCP stream. The message requests that the remote node send the block with the specified hash.
    /// If an error occurs while sending the message, this function returns a `NodeError` containing
    /// the error message.
    ///
    /// # Arguments
    ///
    /// * `stream`: A mutable reference to the `TcpStream` to which to send the message.
    /// * `hash_bytes`: The 32-byte hash of the block to request.
    ///
    /// # Errors
    ///
    /// This function returns a `NodeError` if an error occurs while sending the message.
    pub fn send_message(&self, stream: &mut TcpStream) -> Result<(), NodeError> {
        let get_data_message = self.to_bytes()?;
        let header_get_data = Header::create_header(&get_data_message, COMMAND_NAME_GET_DATA)?;
        let mut bytes = vec![];

        bytes.extend(&header_get_data);
        bytes.extend(&get_data_message);

        send_message(stream, bytes)
    }

    /// Receives a "GetData" message from the given TCP stream.
    pub fn from_stream(
        stream: &mut TcpStream,
        header: &mut Header,
    ) -> Result<GetDataMessage, NodeError> {
        let get_data_bytes = receive_message(stream, header.payload_size())?;
        InvMessage::from_bytes(&get_data_bytes)
    }
}
