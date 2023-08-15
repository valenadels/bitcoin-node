use std::net::TcpStream;

use crate::{
    connectors::peer_connector::receive_message,
    constants::{
        COMMAND_NAME_ADDR, COMMAND_NAME_BLOCK, COMMAND_NAME_FEEFILTER, COMMAND_NAME_GETHEADERS,
        COMMAND_NAME_GET_DATA, COMMAND_NAME_HEADERS, COMMAND_NAME_INV, COMMAND_NAME_NOTFOUND,
        COMMAND_NAME_PING, COMMAND_NAME_PONG, COMMAND_NAME_SENDHEADERS, COMMAND_NAME_TX,
        COMMAND_NAME_VERACK, COMMAND_NAME_VERSION, LENGTH_HEADER_MESSAGE, TESTNET_MAGIC_BYTES,
    },
    node::message_type::MessageType,
    node_error::NodeError,
};

use bitcoin_hashes::{sha256d, Hash};

/// The message header for communication with a Bitcoin node.
///
/// This struct represents the header of a message sent to or received from a Bitcoin node.
#[derive(Debug)]
pub struct Header {
    /// A 4-byte sequence that identifies the message and serves as a protocol version.
    pub start_string: [u8; 4],
    /// The name of the command being sent in ASCII characters. It is later padded with null bytes to reach 12 bytes.
    pub command_name: [u8; 12],
    /// The length of the payload in number of bytes.
    pub payload_size: [u8; 4],
    /// The first 4 bytes of the double SHA-256 hash of the payload.
    pub checksum: [u8; 4],
}

impl Header {
    /// Creates a new message header.
    pub fn new(stream: &mut TcpStream) -> Result<Self, NodeError> {
        let recv_header = receive_message(stream, LENGTH_HEADER_MESSAGE)?;
        let header = Header::from_bytes(&recv_header);
        Ok(header)
    }
    /// Pad a command name with null bytes to make it 12 bytes long.
    ///
    /// If the command name is less than 12 bytes long, it will be padded with null bytes (`0x00`)
    /// to make it exactly 12 bytes long. If it is already 12 bytes long or longer, it will be
    /// truncated to 12 bytes.
    ///
    /// # Arguments
    ///
    /// * `command_name`: A byte slice representing the command name.
    ///
    /// # Returns
    ///
    /// An array of 12 bytes representing the padded command name.
    fn padd_command_name(command_name: &[u8]) -> [u8; 12] {
        let mut padded_command_name = [0u8; 12];
        let len = std::cmp::min(command_name.len(), 12);
        for (i, item) in padded_command_name.iter_mut().enumerate().take(len) {
            *item = *command_name.get(i).unwrap_or(&0);
        }
        padded_command_name
    }

    /// Creates a new header byte array based on the given start string, command name, and payload.
    ///
    /// # Arguments
    ///
    /// * start_string - A 4-byte sequence that identifies the message and serves as a protocol version.
    /// * command_name - The name of the command being sent in ASCII characters. It is later padded with null bytes to reach 12 bytes.
    /// * payload - The data being sent.
    ///
    /// # Returns
    ///
    /// Returns a byte vector containing the header bytes.
    ///
    /// # Errors
    ///
    /// Returns a NodeError::FailedToCreateHeaderField error if there is an issue calculating the checksum.
    fn new_header_bytes(
        start_string: [u8; 4],
        command_name: &[u8],
        payload: &Vec<u8>,
    ) -> Result<Vec<u8>, NodeError> {
        let payload_size: [u8; 4] = (payload.len() as u32).to_le_bytes();
        let mut checksum: &[u8] = &[0x5d, 0xf6, 0xe0, 0xe2];
        let checksum_vec = sha256d::Hash::hash(payload).to_byte_array();
        if payload_size != [0x00, 0x00, 0x00, 0x00] {
            checksum = match checksum_vec.get(0..4) {
                Some(c) => c,
                None => {
                    return Err(NodeError::FailedToCreateHeaderField(
                        "Error while calculating checksum".to_string(),
                    ))
                }
            };
        }

        let mut bytes = vec![];
        bytes.extend(&start_string);

        bytes.extend(Self::padd_command_name(command_name));
        bytes.extend(&payload_size);
        bytes.extend(checksum);

        Ok(bytes)
    }

    /// Creates a new header byte array for the command_name message.
    ///
    /// # Arguments
    ///
    /// * `message_bytes` - The message bytes for the message.
    ///
    /// # Returns
    ///
    /// The header bytes for the message header
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToCreateHeaderField` error if there was a problem creating the header bytes.
    pub fn create_header(
        message_bytes: &Vec<u8>,
        command_name: &str,
    ) -> Result<Vec<u8>, NodeError> {
        Self::new_header_bytes(TESTNET_MAGIC_BYTES, command_name.as_bytes(), message_bytes)
    }

    /// Given a message header in the form of a byte array, returns the size of the payload as a u64.
    ///
    /// # Arguments
    ///
    /// * header - A reference to a byte array with a length of 24, containing the message header.
    pub fn payload_size(&self) -> usize {
        let payload_size = self.payload_size;
        u32::from_le_bytes(payload_size) as usize
    }

    /// Creates a new `Header` instance from the provided byte array.
    ///
    /// # Arguments
    ///
    /// * `header_bytes` - The byte array representing the header.
    ///
    /// # Returns
    ///
    /// A new `Header` instance.
    pub fn from_bytes(header_bytes: &[u8]) -> Header {
        let mut start_string = [0; 4];
        start_string.copy_from_slice(&header_bytes[0..4]);
        let mut command_name = [0; 12];
        command_name.copy_from_slice(&header_bytes[4..16]);
        let mut payload_size = [0; 4];
        payload_size.copy_from_slice(&header_bytes[16..20]);
        let mut checksum = [0; 4];
        checksum.copy_from_slice(&header_bytes[20..24]);
        Header {
            start_string,
            command_name,
            payload_size,
            checksum,
        }
    }

    /// Extracts the command name from the given message header.
    /// This function returns the command name as a String.
    ///
    /// # Arguments
    ///
    /// * `recv_header` - A vector containing the header bytes of the received message.
    ///
    /// # Returns
    ///
    /// A `String` containing the command name.
    ///
    /// # Errors
    ///
    /// This function returns a `NodeError` if it fails to convert the command name bytes to a String.
    pub fn extract_command_name(&mut self) -> Result<MessageType, NodeError> {
        let command_name_bytes = self.command_name;
        let binding = String::from_utf8_lossy(&command_name_bytes);
        let command_name = binding.trim_end_matches('\0');
        match command_name {
            COMMAND_NAME_VERSION => Ok(MessageType::Version),
            COMMAND_NAME_VERACK => Ok(MessageType::Verack),
            COMMAND_NAME_PING => Ok(MessageType::Ping),
            COMMAND_NAME_PONG => Ok(MessageType::Pong),
            COMMAND_NAME_HEADERS => Ok(MessageType::Headers),
            COMMAND_NAME_GETHEADERS => Ok(MessageType::GetHeaders),
            COMMAND_NAME_SENDHEADERS => Ok(MessageType::SendHeaders),
            COMMAND_NAME_ADDR => Ok(MessageType::Addr),
            COMMAND_NAME_FEEFILTER => Ok(MessageType::FeeFilter),
            COMMAND_NAME_INV => Ok(MessageType::Inv),
            COMMAND_NAME_BLOCK => Ok(MessageType::Block),
            COMMAND_NAME_NOTFOUND => Ok(MessageType::NotFound),
            COMMAND_NAME_TX => Ok(MessageType::Tx),
            COMMAND_NAME_GET_DATA => Ok(MessageType::GetData),
            _ => Err(NodeError::CommandTypeError(format!(
                "Unknown command name: {:?}",
                command_name_bytes
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::*;
    use crate::{
        config::load_app_config,
        constants::{COMMAND_NAME_VERSION, TESTNET_MAGIC_BYTES},
        messages::version_message::VersionMessage,
    };

    #[test]
    fn test_create_header() -> Result<(), NodeError> {
        load_app_config(None)?;
        let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8223);
        let version = VersionMessage::new(&ip)?;
        let version_message_bytes = VersionMessage::to_bytes(&version);
        let header_bytes = Header::new_header_bytes(
            TESTNET_MAGIC_BYTES,
            COMMAND_NAME_VERSION.as_bytes(),
            &version_message_bytes,
        )
        .unwrap();
        let header = Header::from_bytes(&header_bytes);
        assert_eq!(header_bytes.len(), 24);
        assert_eq!(header.payload_size(), version_message_bytes.len());
        assert_eq!(header.start_string, TESTNET_MAGIC_BYTES);
        assert_eq!(
            &header.command_name[0..7],
            COMMAND_NAME_VERSION.as_bytes(),
            "Command name does not match"
        );
        let checksum_vec = sha256d::Hash::hash(&version_message_bytes).to_byte_array();
        match checksum_vec.get(0..4) {
            Some(c) => assert_eq!(header.checksum, c, "Checksum does not match"),
            None => {
                return Err(NodeError::FailedToCreateHeaderField(
                    "Error al calcular el checksum".to_string(),
                ))
            }
        };
        Ok(())
    }

    #[test]
    fn test_extract_command_name_version() -> Result<(), NodeError> {
        let empty_payload = [0u8; 12].to_vec();
        let header_message = Header::create_header(&empty_payload, COMMAND_NAME_VERSION)?;
        let mut header = Header::from_bytes(&header_message);
        match header.extract_command_name() {
            Ok(message_type) => match message_type {
                MessageType::Version => assert!(true),
                _ => assert!(false),
            },
            Err(e) => return Err(e),
        }

        Ok(())
    }

    #[test]
    fn test_extract_command_name_getheaders() -> Result<(), NodeError> {
        let empty_payload = [0u8; 12].to_vec();
        let header_message = Header::create_header(&empty_payload, COMMAND_NAME_GETHEADERS)?;
        let mut header = Header::from_bytes(&header_message);
        match header.extract_command_name() {
            Ok(message_type) => match message_type {
                MessageType::GetHeaders => assert!(true),
                _ => assert!(false),
            },
            Err(e) => return Err(e),
        }

        Ok(())
    }

    #[test]
    fn test_extract_command_name_headers() -> Result<(), NodeError> {
        let empty_payload = [0u8; 12].to_vec();
        let header_message = Header::create_header(&empty_payload, COMMAND_NAME_HEADERS)?;
        let mut header = Header::from_bytes(&header_message);
        match header.extract_command_name() {
            Ok(message_type) => match message_type {
                MessageType::Headers => assert!(true),
                _ => assert!(false),
            },
            Err(e) => return Err(e),
        }

        Ok(())
    }
}
