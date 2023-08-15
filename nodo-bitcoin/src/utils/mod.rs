use std::{
    io,
    net::{Ipv4Addr, SocketAddr, TcpStream},
};

use crate::{constants::LENGTH_IP, node_error::NodeError};

use crate::constants::HEXADECIMAL_DIGITS_BASE;

pub struct Utils;

impl Utils {
    /// Reads a variable-length integer (varint) from the given byte slice.
    ///
    /// The function parses the varint encoding according to the Bitcoin protocol.
    /// It returns a tuple containing the decoded integer value and the number of bytes consumed.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The byte slice to read the varint from.
    ///
    /// # Returns
    ///
    /// A result containing the decoded integer value and the number of bytes consumed.
    /// If an error occurs during parsing, an `Err` variant is returned with a corresponding `NodeError`.
    ///
    /// # Errors
    ///
    /// The function may return an error in the following cases:
    ///
    /// * If the byte slice is empty or the prefix byte is missing, an `InvalidSizeOfPrefix` error is returned.
    pub fn read_varint(bytes: &[u8]) -> Result<(u64, usize), NodeError> {
        let prefix = bytes.first().ok_or(NodeError::InvalidSizeOfPrefix(
            "Unexpected end of message".to_string(),
        ))?;
        match prefix {
            0xFD => {
                let mut buf = [0u8; 2];
                buf.copy_from_slice(&bytes[1..3]);
                Ok((u16::from_le_bytes(buf) as u64, 3))
            }
            0xFE => {
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&bytes[1..5]);
                Ok((u32::from_le_bytes(buf) as u64, 5))
            }
            0xFF => {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[1..9]);
                Ok((u64::from_le_bytes(buf), 9))
            }
            n => Ok((*n as u64, 1)),
        }
    }

    /// Convert a SocketAddr to a byte array representing an IPv6 address.
    ///
    /// If the SocketAddr represents an IPv4 address, it will be converted to an IPv6-mapped address.
    ///
    /// # Arguments
    ///
    /// * addr - A reference to a SocketAddr object.
    ///
    /// # Returns
    ///
    /// A [u8; 16] byte array representing the IPv6 address.
    pub fn socket_addr_to_ipv6_bytes(addr: &SocketAddr) -> [u8; 16] {
        let ipv6_bytes: [u8; 16] = match addr {
            SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped().octets(),
            SocketAddr::V6(v6) => v6.ip().octets(),
        };
        ipv6_bytes
    }

    /// Checks if a TcpStream is connected.
    pub fn is_tcpstream_connected(stream: &TcpStream) -> bool {
        match stream.peer_addr() {
            Ok(_) => true,
            Err(err) => err.kind() != io::ErrorKind::NotConnected,
        }
    }

    /// Converts a byte vector to a hex string.
    pub fn bytes_to_hex(bytes: &[u8]) -> String {
        let hex_chars: Vec<String> = bytes.iter().map(|byte| format!("{:02x}", byte)).collect();
        hex_chars.join("")
    }

    /// Converts a hex string to a byte vector.
    pub fn hex_string_to_bytes(hex_string: String) -> Result<Vec<u8>, NodeError> {
        let hex_chars: Vec<char> = hex_string.chars().collect();

        if hex_chars.len() % 2 != 0 {
            return Err(NodeError::InvalidHexString(
                "Invalid hex string length".to_string(),
            ));
        }

        let mut bytes = Vec::with_capacity(hex_chars.len() / 2);

        let mut i = 0;
        while i < hex_chars.len() {
            let hex_digit1 = match hex_chars[i].to_digit(HEXADECIMAL_DIGITS_BASE) {
                Some(digit) => digit,
                None => {
                    return Err(NodeError::InvalidHexString(
                        "Invalid character in hex string".to_string(),
                    ))
                }
            };

            let hex_digit2 = match hex_chars[i + 1].to_digit(HEXADECIMAL_DIGITS_BASE) {
                Some(digit) => digit,
                None => {
                    return Err(NodeError::InvalidHexString(
                        "Invalid character in hex string".to_string(),
                    ))
                }
            };

            let byte = ((hex_digit1 << 4) | hex_digit2) as u8;
            bytes.push(byte);

            i += 2;
        }

        Ok(bytes)
    }

    /// Converts a `Vec<u8>` representing an IPv4 address to a `SocketAddr`.
    ///
    /// # Arguments
    ///
    /// * `ip` - A `Vec<u8>` containing the bytes of the IPv4 address.
    /// * `port` - The port number for the `SocketAddr`.
    ///
    /// # Errors
    ///
    /// Returns a `NodeError::FailedToParse` variant if the `ip` length is not equal to 4,
    /// indicating an invalid IP address format.
    pub fn vec_u8_to_socket_addr(ip: Vec<u8>, port: u16) -> Result<SocketAddr, NodeError> {
        if ip.len() == LENGTH_IP {
            let ipv4_addr = Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
            Ok(SocketAddr::new(ipv4_addr.into(), port))
        } else {
            Err(NodeError::FailedToParse(
                "Invalid IP address format in version message".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    use crate::utils::Utils;

    #[test]
    fn test_socket_addr_to_ipv6_bytes() {
        let ipv4_addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080);
        let expected_ipv6_bytes = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 127, 0, 0, 1];

        let result_ipv6_bytes = Utils::socket_addr_to_ipv6_bytes(&ipv4_addr);
        assert_eq!(result_ipv6_bytes, expected_ipv6_bytes);

        let ipv6_addr = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), 8080);
        let expected_ipv6_bytes = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

        let result_ipv6_bytes = Utils::socket_addr_to_ipv6_bytes(&ipv6_addr);
        assert_eq!(result_ipv6_bytes, expected_ipv6_bytes);
    }
}
