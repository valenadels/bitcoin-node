use std::{
    net::{SocketAddr, ToSocketAddrs},
    vec::IntoIter,
};

use crate::node_error::NodeError;

/// A struct representing a DNS connector.
///
/// The `DNSConnector` struct contains the `dns` hostname and `port` number used to connect to a DNS server.
///
pub struct DNSConnector {
    pub dns: String,
    pub port: u16,
}

impl DNSConnector {
    /// Create a new DNS connector.
    ///
    /// This function creates a new `DNSConnector` instance with the given `dns` hostname and `port` number.
    pub fn new(dns: String, port: u16) -> DNSConnector {
        DNSConnector { dns, port }
    }

    /// Connect to the DNS server.
    ///
    /// This function performs a DNS lookup using the `dns` and `port` values provided when the `DNSConnector` instance was created and returns an iterator over `SocketAddr` values representing the addresses of the DNS server.
    ///
    /// # Errors
    ///
    /// Returns an error if the DNS lookup fails.
    pub fn connect(&self) -> Result<IntoIter<SocketAddr>, NodeError> {
        match format!("{}:{}", self.dns, self.port).to_socket_addrs() {
            Ok(addrs) => Ok(addrs),
            Err(_) => Err(NodeError::FailedToConnectDNS(
                "Failed to perform DNS lookup".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dnsconnector() {
        let dns = "seed.testnet.bitcoin.sprovoost.nl";
        let port = 18333;

        let dnsconnector = DNSConnector::new(dns.to_string(), port);
        let ips = dnsconnector.connect();
        assert!(ips.is_ok());
    }

    #[test]
    fn test_dnsconnector_error() {
        let dns = "dsgfg<";
        let port = 0;

        let dnsconnector = DNSConnector::new(dns.to_string(), port);
        dnsconnector
            .connect()
            .expect_err("Failed to perform DNS lookup");
    }
}
