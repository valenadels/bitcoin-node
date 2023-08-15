use std::{
    io::{Read, Write},
    net::TcpStream,
};

use crate::node_error::NodeError;

/// Sends a message over the given TCP stream.
///
/// # Arguments
///
/// * stream - A mutable reference to a TcpStream over which to send the message.
/// * message - A vector of bytes containing the message to send.
///
/// # Errors
///
/// Returns a NodeError::FailedToSendMessage if the message fails to send.
pub fn send_message(stream: &mut TcpStream, message: Vec<u8>) -> Result<(), NodeError> {
    stream
        .write_all(&message)
        .map_err(|e| NodeError::FailedToSendMessage(format!("Failed to send message: {}", e)))?;
    Ok(())
}

/// Reads a message from the given readable source
///
/// # Arguments
///
/// * source - A mutable reference to a source implementing the Read trait from which to read the message.
/// * length - Length of the message received
///
/// #Errors
///
/// Returns a NodeError::FailedToSendMessage if the function fails to receive the message.
pub fn receive_message<R: Read>(source: &mut R, length: usize) -> Result<Vec<u8>, NodeError> {
    let mut received_message = vec![0; length];

    match source.read_exact(&mut received_message) {
        Ok(_) => Ok(received_message),
        Err(e) => Err(NodeError::FailedToReceiveMessage(format!(
            "Failed to receive message: {}",
            e
        ))),
    }
}
/// Receives a block message from a TCP stream.
///
/// This function reads a specified number of bytes from the provided TCP stream and returns
/// the received message as a vector of bytes (`Vec<u8>`).
///
/// # Arguments
///
/// * `stream` - A mutable reference to a `TcpStream` representing the network stream from which to receive the message.
/// * `length` - The expected length of the message to receive, in bytes.
///
/// # Returns
///
/// Returns a `Result` indicating the success or failure of receiving the block message.
///
/// If the message is successfully received and has the expected length, the received message
/// is returned as a `Vec<u8>` within the `Ok` variant of the `Result`.
///
/// If there is an error while receiving the message, an appropriate `NodeError` is returned
/// within the `Err` variant of the `Result`.
///
/// # Errors
///
/// This function can return an error in the following cases:
///
/// * If the number of bytes read from the stream does not match the expected length, a `NodeError`
///   is returned with a descriptive error message indicating the number of bytes read, the expected
///   number of bytes, and the buffer containing the received message.
///
/// * If there is an underlying I/O error while reading from the stream, a `NodeError` is returned
///   with a descriptive error message indicating the underlying error.

pub fn receive_block_message(stream: &mut TcpStream, length: usize) -> Result<Vec<u8>, NodeError> {
    let mut received_message = vec![0; length];

    match stream.read(&mut received_message) {
        Ok(bytes_leidos) => {
            if bytes_leidos == length {
                return Ok(received_message);
            }

            Err(NodeError::FailedToReceiveMessage(format!(
                "The expected number of bytes were not read. Bytes read: {}, Bytes expected to be read: {}, Buffer: {:?}", bytes_leidos, length,received_message)))
        }
        Err(e) => Err(NodeError::FailedToReceiveMessage(format!(
            "Failed to receive message: {}",
            e
        ))),
    }
}
