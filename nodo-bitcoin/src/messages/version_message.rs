use crate::compact_size::CompactSize;
use crate::connectors::peer_connector::send_message;
use crate::constants::{COMMAND_NAME_VERSION, LOCAL_IP, LOCAL_PORT};
use crate::header::Header;
use crate::node::read::retrieve_version;
use crate::node_error::NodeError;
use crate::utils::Utils;

use chrono::Utc;
use rand::Rng;
use std::net::TcpStream;
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

/// Represents the version message that is sent during the handshake process between nodes.
#[derive(Debug, PartialEq)]
pub struct VersionMessage {
    /// The highest protocol version understood by the transmitting node.
    pub version: i32,
    /// The services supported by the transmitting node.
    pub services: u64,
    /// The timestamp of the version message.
    pub timestamp: i64,
    /// The services supported by the receiving node as perceived by the transmitting node.
    pub addr_recv_services: u64,
    /// The IPv6 address of the receiving node as perceived by the transmitting node.
    pub addr_recv_address: [u8; 16],
    /// The port number of the receiving node as perceived by the transmitting node.
    pub addr_recv_port: u16,
    /// The services supported by the transmitting node.
    pub addr_trans_services: u64,
    /// The IPv6 address of the transmitting node.
    pub addr_trans_addr: [u8; 16],
    /// The port number of the transmitting node.
    pub addr_trans_port: u16,
    /// A random nonce used to detect connections to self.
    pub nonce: u64,
    /// The user agent of the transmitting node.
    pub user_agent_bytes: u8,
    /// The last block received by the transmitting node.
    pub start_height: i32,
    /// Whether the transmitting node wants to receive inv messages for transactions.
    pub relay: u8,
}

impl VersionMessage {
    /// Constructs a new VersionMessage struct using the given peer_addr as the address of the peer node.
    ///
    /// # Arguments
    ///
    /// * peer_addr - A SocketAddr struct representing the address of the peer node.
    ///
    /// # Errors
    ///
    /// Returns a NodeError if there was an issue getting the local socket address.
    pub fn new(peer_addr: &SocketAddr) -> Result<VersionMessage, NodeError> {
        let local_ip = Self::get_local_socket_addr()?;

        Ok(VersionMessage {
            version: retrieve_version(),
            services: 0,
            timestamp: Utc::now().timestamp(),
            addr_recv_services: 1,
            addr_recv_address: Utils::socket_addr_to_ipv6_bytes(peer_addr),
            addr_recv_port: peer_addr.port(),
            addr_trans_addr: Utils::socket_addr_to_ipv6_bytes(&local_ip),
            addr_trans_port: LOCAL_PORT,
            nonce: rand::thread_rng().gen(),
            user_agent_bytes: 0,
            start_height: 0,
            addr_trans_services: 0,
            relay: 1,
        })
    }

    /// Returns the local socket address from the environment variables.
    ///
    /// # Errors
    ///
    /// Returns an EnvironVarNotFound error if the LOCAL_IP variable is not found.
    /// Returns a FailedToParse error if the LOCAL_IP value can't be parsed to an IpAddr.
    fn get_local_socket_addr() -> Result<SocketAddr, NodeError> {
        let local_ip_str = std::env::var(LOCAL_IP)
            .map_err(|_| NodeError::EnvironVarNotFound("Local ip no found".to_string()))?;

        let ip_addr = IpAddr::from_str(&local_ip_str)
            .map_err(|_| NodeError::FailedToParse("Failed to parse IP address".to_string()))?;

        let socket_addr = SocketAddr::new(ip_addr, LOCAL_PORT);

        Ok(socket_addr)
    }

    /// Converts a VersionMessage struct to a byte vector.
    ///
    /// # Arguments
    ///
    /// * version_message - A reference to a VersionMessage struct.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(&self.version.to_le_bytes());
        bytes.extend(&self.services.to_le_bytes());
        bytes.extend(&self.timestamp.to_le_bytes());
        bytes.extend(&self.addr_recv_services.to_le_bytes());
        bytes.extend(&self.addr_recv_address);
        bytes.extend(&self.addr_recv_port.to_le_bytes());
        bytes.extend(&self.addr_trans_services.to_le_bytes());
        bytes.extend(&self.addr_trans_addr);
        bytes.extend(&self.addr_trans_port.to_le_bytes());
        bytes.extend(&self.nonce.to_le_bytes());
        bytes.extend(&self.user_agent_bytes.to_le_bytes());
        bytes.extend(&self.start_height.to_le_bytes());
        bytes.extend(&self.relay.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<VersionMessage, NodeError> {
        let version = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let services = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
        let timestamp = i64::from_le_bytes(bytes[12..20].try_into().unwrap());
        let addr_recv_services = u64::from_le_bytes(bytes[20..28].try_into().unwrap());
        let addr_recv_address = bytes[28..44].try_into().unwrap();
        let addr_recv_port = u16::from_be_bytes(bytes[44..46].try_into().unwrap());
        let addr_trans_services = u64::from_le_bytes(bytes[46..54].try_into().unwrap());
        let addr_trans_addr = bytes[54..70].try_into().unwrap();
        let addr_trans_port = u16::from_be_bytes(bytes[70..72].try_into().unwrap());
        let nonce = u64::from_le_bytes(bytes[72..80].try_into().unwrap());
        let user_agent_bytes = CompactSize::read_varint(&mut &bytes[80..])?.get_value() as usize;
        let start_height = i32::from_le_bytes(
            bytes[80 + user_agent_bytes..84 + user_agent_bytes]
                .try_into()
                .unwrap(),
        );
        let relay = u8::from_be_bytes(
            bytes[84 + user_agent_bytes..85 + user_agent_bytes]
                .try_into()
                .unwrap(),
        );

        Ok(VersionMessage {
            version,
            services,
            timestamp,
            addr_recv_services,
            addr_recv_address,
            addr_recv_port,
            addr_trans_services,
            addr_trans_addr,
            addr_trans_port,
            nonce,
            user_agent_bytes: user_agent_bytes as u8,
            start_height,
            relay,
        })
    }

    /// Creates a new version message for the given SocketAddr.
    ///
    /// # Arguments
    ///
    /// * ip - The SocketAddr representing the IP address and port of the node to send the message to.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the newly created VersionMessage on success, or a NodeError on failure.
    ///
    /// # Errors
    ///
    /// Returns a NodeError if it fails to create the version message.
    pub fn create_version_message(ip: &SocketAddr) -> Result<VersionMessage, NodeError> {
        let version_message = VersionMessage::new(ip).map_err(|_| {
            NodeError::FailedToCreateVersionMessage("Failed to create version message".to_string())
        })?;
        Ok(version_message)
    }

    /// Sends the version message to the given TcpStream.
    ///
    /// # Arguments
    ///
    /// * stream - A mutable reference to a TcpStream.
    ///
    /// # Returns
    ///
    /// Returns a Result containing () on success, or a NodeError on failure.
    pub fn send_message(&self, stream: &mut TcpStream) -> Result<(), NodeError> {
        let version_message_bytes = self.to_bytes();
        let header_version = Header::create_header(&version_message_bytes, COMMAND_NAME_VERSION)?;

        let mut bytes = vec![];

        bytes.extend(header_version);
        bytes.extend(version_message_bytes);
        send_message(stream, bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::config::load_app_config;
    use crate::constants::LOCAL_PORT;
    use crate::node::read::retrieve_version;

    #[test]
    fn test_version_message_creation() -> Result<(), NodeError> {
        load_app_config(None)?;
        let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8223);
        let version = VersionMessage::new(&ip)?;
        let local_ip = VersionMessage::get_local_socket_addr()?;

        let current_version = retrieve_version();
        assert_eq!(version.version, current_version);
        assert_eq!(version.services, 0);
        assert_eq!(version.addr_recv_services, 1);
        assert_eq!(
            version.addr_recv_address,
            Utils::socket_addr_to_ipv6_bytes(&ip)
        );
        assert_eq!(version.addr_recv_port, ip.port());
        assert_eq!(version.user_agent_bytes, 0);
        assert_eq!(version.start_height, 0);
        assert_eq!(version.addr_trans_port, LOCAL_PORT);
        assert_eq!(
            version.addr_trans_addr,
            Utils::socket_addr_to_ipv6_bytes(&local_ip)
        );
        Ok(())
    }

    #[test]
    fn version_message_to_bytes_test() -> Result<(), NodeError> {
        load_app_config(None)?;
        let ip = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8223);
        let version = VersionMessage::new(&ip)?;

        let bytes = VersionMessage::to_bytes(&version);
        assert!(bytes.len() > 84);
        Ok(())
    }
}
