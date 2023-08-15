use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, Write},
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
    vec::IntoIter,
};

use crate::{
    block_header::{block_header_bytes::BlockHeaderBytes, BlockHeader, GENESIS_BLOCK_HEADER},
    config::obtain_dir_path,
    connectors::dns_connector::DNSConnector,
    constants::{
        BLOCK_HEADERS_FILE, DEFAULT_VERSION, DNS, LENGTH_BLOCK_HEADERS, PORT, STARTING_DATE,
        VERSION,
    },
    node_error::NodeError,
};

/// Converts an `IntoIter<SocketAddr>` into a `Vec<SocketAddr>`.
///
/// # Arguments
///
/// * `iter_ips` - An iterator over `SocketAddr` values.
///
/// # Returns
///
/// Returns a `Result` containing the converted `Vec<SocketAddr>` if successful,
/// or an `Err` variant with a `NodeError` if an error occurs during the conversion.
pub fn turn_iter_into_vector(iter_ips: IntoIter<SocketAddr>) -> Vec<SocketAddr> {
    let mut ips = Vec::new();
    for ip in iter_ips {
        ips.push(ip);
    }
    ips
}

/// Adds IP addresses and ports specified in the "PEER_IPS" environment variable to the given vector of `SocketAddr`.
///
/// If the "PEER_IPS" environment variable is empty, the function returns early without modifying the vector.
/// Otherwise, it parses the IP addresses and ports from the "PEER_IPS" string, constructs `SocketAddr` instances,
/// and adds them to the vector.
///
/// # Arguments
///
/// * `ips` - A mutable reference to a vector of `SocketAddr` to which the IP addresses and ports will be added.
///
/// # Returns
///
/// A `Result` indicating success or an `NodeError` if an error occurs during parsing.
///
/// # Errors
///
/// The function can return a `NodeError` if parsing the IP addresses or ports fails.
fn add_config_ips(ips: &mut Vec<SocketAddr>) -> Result<(), NodeError> {
    let peer_ips = std::env::var("PEER_IPS")
        .map_err(|_| NodeError::EnvironVarNotFound("PEER_IPS not found in env vars".to_string()))?;

    if peer_ips.is_empty() {
        return Ok(());
    }

    let new_ips: Vec<_> = peer_ips
        .split(',')
        .filter_map(|ip_port| {
            let parts: Vec<_> = ip_port.split(':').collect();
            if parts.len() == 2 {
                let ip_str = parts[0].trim();
                let ip = match Ipv4Addr::from_str(ip_str) {
                    Ok(ip) => ip,
                    Err(_) => return None,
                };
                let port = match parts[1].trim().parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => return None,
                };

                Some(SocketAddr::new(std::net::IpAddr::V4(ip), port))
            } else {
                None
            }
        })
        .collect();

    println!("Adding IPs from PEER_IPS: {:?}", new_ips);
    ips.splice(0..0, new_ips);
    Ok(())
}

/// Returns the list of IP addresses obtained from DNS lookup using the DNS and PORT environment variables.
///
/// # Errors
///
/// Returns a NodeError::EnvironVarNotFound error if the DNS or PORT environment variables are not set.
///
/// Returns a NodeError::FailedToParse error if the PORT environment variable is not a valid u16 value.
pub fn obtain_ips() -> Result<Vec<SocketAddr>, NodeError> {
    let dns = std::env::var(DNS)
        .map_err(|_| NodeError::EnvironVarNotFound("DNS not found in env vars".to_string()))?;
    let port = std::env::var(PORT)
        .map_err(|_| NodeError::EnvironVarNotFound("PORT not found in env vars".to_string()))?
        .parse::<u16>()
        .map_err(|_| NodeError::FailedToParse("Invalid PORT format in env vars".to_string()))?;
    let dns_connector = DNSConnector::new(dns, port);
    let mut ips = turn_iter_into_vector(dns_connector.connect()?);
    add_config_ips(&mut ips)?;

    Ok(ips)
}

/// Retrieves the version field of the VersionMessage from the environment variable VERSION, and returns it as an integer.
/// If the VERSION environment variable is not found or cannot be parsed to an integer, the default value DEFAULT_VERSION (70015) is returned.
pub fn retrieve_version() -> i32 {
    let version = std::env::var(VERSION).map_err(|_| "VERSION not found in env vars".to_string());
    match version {
        Ok(version) => version
            .parse::<i32>()
            .map_err(|_| "Invalid VERSION format in env vars".to_string())
            .unwrap_or(DEFAULT_VERSION),
        Err(_) => DEFAULT_VERSION,
    }
}

/// Reads the starting date from the environment variable STARTING_DATE and returns it as a u32.
///
/// # Errors
///
/// Returns a NodeError::FailedToLoadConfig error if the STARTING_DATE environment variable is not found.
pub fn read_timestamp() -> Result<u32, NodeError> {
    let timestamp = std::env::var(STARTING_DATE).map_err(|_| {
        NodeError::FailedToLoadConfig("STARTING_DATE not found in env vars".to_string())
    })?;
    let timestamp = timestamp
        .parse::<u32>()
        .map_err(|_| NodeError::FailedToParse("Invalid STARTING_DATE format".to_string()))?;
    Ok(timestamp)
}

/// Reads the last block header from the block headers file and returns it as a byte vector.
/// If the block headers file is empty, the genesis block header is returned.
///
/// # Errors
///
/// Returns a NodeError::FailedToOpenFile error if the block headers file cannot be opened.
///
/// Returns a NodeError::FailedToRead error if the block headers file cannot be read.
///
/// Returns a NodeError::FailedToSeek error if the block headers file cannot be seeked.
pub fn read_last_block_header() -> Result<BlockHeaderBytes, NodeError> {
    let dir_headers_file = obtain_dir_path(BLOCK_HEADERS_FILE.to_owned())?;
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(dir_headers_file)
        .map_err(|_| NodeError::FailedToOpenFile("Failed to open headers file".to_string()))?;

    let file_size = file
        .seek(io::SeekFrom::End(0))
        .map_err(|_| NodeError::FailedToRead("Failed to seek end of file".to_string()))?;

    let mut pos = 0;

    if file_size >= LENGTH_BLOCK_HEADERS as u64 {
        pos = file_size - LENGTH_BLOCK_HEADERS as u64
    }

    if pos == 0 {
        println!("Writing genesis block header to file");
        file.write_all(&GENESIS_BLOCK_HEADER.to_bytes())
            .map_err(|_| NodeError::FailedToWrite("Failed to write to file".to_string()))?;
        return Ok(GENESIS_BLOCK_HEADER.to_bytes());
    }

    file.seek(io::SeekFrom::Start(pos)).map_err(|_| {
        NodeError::FailedToRead("Failed to seek position while reading from file".to_string())
    })?;
    let mut buffer = [0u8; LENGTH_BLOCK_HEADERS];
    file.read_exact(&mut buffer)
        .map_err(|_| NodeError::FailedToRead("Failed to read exact from file".to_string()))?;
    Ok(buffer.to_vec())
}

/// Reads the initial block headers from a file containing block header bytes.
///
/// # Returns
///
/// A `Vec<BlockHeader>` containing the block headers read from the file.
/// These are filterd as their timestamp is greater than the one from the start of the project.
///
/// # Errors
///
/// Returns an error of type `NodeError` if there is a problem reading or parsing
/// the block header bytes.
pub fn read_initial_block_headers_from_file() -> Result<Vec<BlockHeader>, NodeError> {
    let dir_headers_file = obtain_dir_path(BLOCK_HEADERS_FILE.to_owned())?;
    let mut file = OpenOptions::new()
        .read(true)
        .open(dir_headers_file)
        .map_err(|_| {
            NodeError::FailedToOpenFile("Failed to open block headers file".to_string())
        })?;

    println!("Getting initial block headers from file");

    let mut initial_block_headers = Vec::new();

    let file_size = file
        .seek(io::SeekFrom::End(0))
        .map_err(|_| NodeError::FailedToRead("Failed to seek end of file".to_string()))?;

    let pos = file_size - LENGTH_BLOCK_HEADERS as u64;
    read_block_headers(pos, file, &mut initial_block_headers)?;

    initial_block_headers.reverse();

    Ok(initial_block_headers)
}
/// Reads block headers from a file and populates the initial block headers vector.
///
/// # Arguments
///
/// * `pos` - The initial position from which to start reading in the file.
/// * `file` - A mutable reference to a `File` to read from.
/// * `initial_block_headers` - A mutable reference to a vector of `BlockHeader` instances
///                             to store the read block headers.
///
/// # Returns
///
/// Returns `Ok(())` if the block headers are successfully read and stored,
/// or a `NodeError` on failure.
///
/// # Errors
///
/// Returns a `NodeError::FailedToRead` variant if there was an error while seeking or reading from the file.
/// Returns a `NodeError` if there was an error while converting the bytes to `BlockHeader`.
fn read_block_headers(
    mut pos: u64,
    mut file: File,
    initial_block_headers: &mut Vec<BlockHeader>,
) -> Result<(), NodeError> {
    let starting_timestamp = read_timestamp()?;

    while pos > 0 {
        file.seek(io::SeekFrom::Start(pos)).map_err(|_| {
            NodeError::FailedToRead("Failed to seek position while reading from file".to_string())
        })?;
        let mut buffer = [0u8; LENGTH_BLOCK_HEADERS].to_vec();
        file.read_exact(&mut buffer)
            .map_err(|_| NodeError::FailedToRead("Failed to read exact from file".to_string()))?;

        let block_header = BlockHeader::from_bytes(&buffer)?;

        if block_header.timestamp < starting_timestamp {
            break;
        }

        initial_block_headers.push(block_header);

        pos -= LENGTH_BLOCK_HEADERS as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::net::IpAddr;

    use crate::messages::block_message::BlockMessage;

    use super::*;

    #[test]
    fn test_get_block_path() -> Result<(), NodeError> {
        let block_header = BlockHeader {
            version: 556843008,
            prev_blockhash: [
                205, 177, 222, 128, 213, 159, 58, 96, 24, 113, 15, 235, 116, 46, 241, 3, 39, 237,
                39, 66, 169, 125, 63, 141, 24, 0, 0, 0, 0, 0, 0, 0,
            ],
            merkle_root_hash: [
                7, 158, 112, 190, 109, 133, 214, 92, 71, 104, 1, 99, 172, 188, 135, 237, 152, 82,
                32, 253, 152, 245, 155, 38, 57, 4, 13, 45, 78, 247, 247, 216,
            ],
            timestamp: 1683921494,
            n_bits: 486604799,
            nonce: 3685783874,
            hash: [
                243, 200, 175, 162, 222, 36, 17, 224, 203, 218, 152, 71, 85, 159, 228, 254, 184,
                211, 188, 93, 247, 77, 196, 77, 181, 75, 0, 0, 0, 0, 0, 0,
            ]
            .to_vec(),
        };

        let block_path =
            match BlockMessage::block_path(block_header.hash().as_slice().try_into().unwrap()) {
                Some(path) => path,
                None => {
                    return Err(NodeError::FailedToRead(
                        "Failed to get block path".to_string(),
                    ))
                }
            };

        assert_eq!(
            block_path,
            "blocks/0000000000004bb54dc44df75dbcd3b8fee49f554798dacbe01124dea2afc8f3.bin"
        );

        Ok(())
    }

    #[test]
    fn test_add_config_ips() {
        let mut ips = Vec::new();
        let peer_ips = "192.168.0.1:8080, 10.0.0.1:12345, 127.0.0.1:9999";
        std::env::set_var("PEER_IPS", peer_ips);
        add_config_ips(&mut ips).unwrap();

        let expected_ips = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 8080),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 12345),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9999),
        ];
        assert_eq!(ips, expected_ips);
    }
}
