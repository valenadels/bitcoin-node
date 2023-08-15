use std::{
    fs::OpenOptions,
    net::{TcpListener, TcpStream},
    thread,
};

use crate::{
    config::obtain_dir_path,
    connectors::peer_connector::{receive_message, send_message},
    constants::{BLOCK_HEADERS_FILE, LENGTH_HEADER_MESSAGE, LOCAL_IP, PORT},
    header::Header,
    messages::{
        block_message::BlockMessage,
        get_data_message::GetDataMessage,
        get_headers_message::GetHeadersMessage,
        headers_message::HeadersMessage,
        verack_message::{is_verack_message, VERACK_MESSAGE},
        version_message::VersionMessage,
    },
    node::message_type::MessageType,
    node_error::NodeError,
    utils::Utils,
};

/// Starts the server and listens for incoming client connections.
///
/// The function reads the port number from the `PORT` environment variable,
/// binds a `TcpListener` to the specified port, and listens for incoming
/// client connections. For each incoming connection, it spawns a new thread
/// to handle the client connection by calling the `handle_client` function.
///
/// # Errors
///
/// Returns a `Result` indicating whether the server was started successfully
/// (`Ok(())`) or an error occurred during the server startup process (`Err`).
pub fn start_server() -> Result<(), NodeError> {
    let port = std::env::var(PORT)
        .map_err(|_| NodeError::EnvironVarNotFound("PORT not found in env vars".to_string()))?
        .parse::<u16>()
        .map_err(|_| NodeError::FailedToParse("Invalid PORT format in env vars".to_string()))?;
    let addr = std::env::var(LOCAL_IP)
        .map_err(|_| NodeError::EnvironVarNotFound("Local ip no found".to_string()))?;

    let listener = TcpListener::bind((addr, port))
        .map_err(|_| NodeError::FailedToBind(format!("Failed to bind to port {}", port)))?;

    println!("Server started, listening on port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| match handle_client(stream) {
                    Ok(_) => println!("Connection processed!"),
                    Err(e) => {
                        println!("Failed to process connection: {:?}", e);
                    }
                });
            }
            Err(_) => {
                "Failed to read from stream".to_string();
            }
        }
    }
    Ok(())
}

/// Handles incoming messages from the connected stream, processing each message based on its command type.
///
/// # Arguments
///
/// * `stream` - A mutable reference to a TCP stream representing the connection to the node.
///
/// # Returns
///
/// * `Ok(())` - If the message handling loop completes successfully.
/// * `Err(NodeError)` - If there is an error while receiving or processing the messages.
pub fn client_message_handler(stream: &mut TcpStream) -> Result<(), NodeError> {
    let dir_headers_file = obtain_dir_path(BLOCK_HEADERS_FILE.to_owned())?;
    let mut headers_file = OpenOptions::new()
        .read(true)
        .open(dir_headers_file)
        .map_err(|_| {
            NodeError::FailedToOpenFile("Failed to open block headers file".to_string())
        })?;
    let mut count_getdata = 1;
    let mut count_headers = 1;

    loop {
        if !Utils::is_tcpstream_connected(stream) {
            println!("Tcp stream {:?} is not connected", stream.peer_addr());
            return Ok(());
        }
        let mut header = Header::new(stream).map_err(|e| {
            NodeError::FailedToReadExact(format!(
                "Failed stream: {:?}: {:?}",
                stream.peer_addr(),
                e
            ))
        })?;
        let command_name = header.extract_command_name()?;

        match command_name {
            MessageType::GetHeaders => {
                println!(
                    "Receiving command: {:?}, count header messages: {:?}",
                    command_name, count_headers
                );
                let get_headers_message = GetHeadersMessage::from_stream(stream)?;
                HeadersMessage::send_batch_headers(stream, get_headers_message, &mut headers_file)?;
                count_headers += 1;
            }
            MessageType::GetData => {
                println!(
                    "Receiving command: {:?}, count getdata messages: {:?}",
                    command_name, count_getdata
                );
                let getdata_message = GetDataMessage::from_stream(stream, &mut header)?;
                BlockMessage::send_message(stream, getdata_message)?;
                count_getdata += 1;
            }
            _ => {
                println!("Command not supported");
                receive_message(stream, header.payload_size())?;
            }
        }
    }
}

/// Handles a client connection by performing the server-side handshake and handling incoming messages.
///
/// The function performs the server-side handshake with the client represented by the
/// `stream`. It calls the `server_handshake` function to perform the handshake and
/// prints a message indicating that the handshake has been completed. It then calls
/// the `client_message_handler` function to handle incoming messages from the client.
///
/// # Arguments
///
/// * `stream` - A mutable reference to a `TcpStream` representing the client connection.
///
/// # Errors
///
/// Returns a `Result` indicating whether the handling of the client connection was successful
/// (`Ok(())`) or an error occurred during the handshake process (`Err`).
fn handle_client(mut stream: TcpStream) -> Result<(), NodeError> {
    if !server_handshake(&mut stream)? {
        println!("Handshake failed with node: {:?}", stream.peer_addr());
        return Err(NodeError::HandshakeFailed(
            "Handshake failed with node".to_string(),
        ));
    }
    println!("Handshake completed with node: {:?}", stream.peer_addr());
    match client_message_handler(&mut stream) {
        Ok(_) => {
            println!("Client message handling completed");
            Ok(())
        }
        Err(e) => {
            println!("Client message handling failed: {:?}", e);
            Err(e)
        }
    }
}

/// Performs the server-side handshake with a peer.
///
/// The function reads the initial handshake message from the `stream` and responds
/// with a version message and a verack message. It also verifies the received verack
/// message to ensure the handshake is successful.
///
/// # Arguments
///
/// * `stream` - A mutable reference to a `TcpStream` representing the connection to the peer.
///
/// # Errors
///
/// Returns a `Result` indicating whether the handshake was successful (`Ok(true)`) or an
/// error occurred during the handshake process (`Ok(false)` or `Err`).
pub fn server_handshake(stream: &mut TcpStream) -> Result<bool, NodeError> {
    let header = Header::new(stream)?;
    let payload_size = header.payload_size();
    let version_peer = receive_message(stream, payload_size)?;

    let ip_bytes = version_peer[66..70].to_vec();
    let port_bytes = version_peer[70..72].to_vec();
    let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);
    let ip = Utils::vec_u8_to_socket_addr(ip_bytes, port).map_err(|_| {
        NodeError::FailedToParse("Invalid IP address format in version message".to_string())
    })?;

    let version_message = VersionMessage::create_version_message(&ip)?;
    version_message.send_message(stream)?;

    let transmiting_ver_ack = VERACK_MESSAGE.to_vec();

    let verack_received = &receive_message(stream, LENGTH_HEADER_MESSAGE)?;
    send_message(stream, transmiting_ver_ack)?;

    Ok(is_verack_message(verack_received))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crate::utils::Utils;

    use super::*;

    #[test]
    fn test_vec_u8_to_socket_addr_ipv4() -> Result<(), NodeError> {
        let ip_bytes = vec![127, 0, 0, 1];
        let port: u16 = 8080;
        let expected_result = Ok(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        ))?;

        // Call the vec_u8_to_socket_addr function
        let result = Utils::vec_u8_to_socket_addr(ip_bytes, port)?;

        // Verify that the result matches the expected result
        assert_eq!(result, expected_result);
        Ok(())
    }
}
