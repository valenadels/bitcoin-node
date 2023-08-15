use crate::{
    block_header::BlockHeader, constants::SAVED_ACCOUNTS, node_error::NodeError,
    wallet::wallet_account_info::AccountInfo,
};
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use glib::Object;
use gtk::{prelude::*, Box, Label};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader},
};

/// Function to convert a vector of bytes to a hex string
pub fn u8_to_hex_string(bytes: &[u8]) -> String {
    let hex_chars: &[u8] = b"0123456789abcdef";
    let mut hex_string = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        hex_string.push(hex_chars[(byte >> 4) as usize] as char);
        hex_string.push(hex_chars[(byte & 0xF) as usize] as char);
    }

    hex_string
}

// Function to create a label with title and info
pub fn create_label_with_title(title: &str, info: &str) -> Label {
    let label_text = format!("<b>{}</b>: {}", title, info);
    let label = Label::new(None);
    label.set_xalign(0.0);
    label.set_markup(&label_text);
    label.set_use_markup(true);
    label
}

/// Function to build a block info box
/// This box contains the block hash, merkle root, previous block hash, nonce, timestamp and version
/// of a block
/// This function is used in the block info page
pub fn build_block_info(block_header: &BlockHeader) -> Box {
    let block_info = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let block_hash = u8_to_hex_string(&block_header.hash);
    let merkle_root = u8_to_hex_string(&block_header.merkle_root_hash);
    let previous_block_hash = u8_to_hex_string(&block_header.prev_blockhash);
    let nonce = block_header.nonce.to_string();
    let timestamp = block_header.timestamp.to_string();
    let version = block_header.version.to_string();

    let block_hash_label = create_label_with_title("Block Hash", &block_hash);
    let merkle_root_label = create_label_with_title("Merkle Root", &merkle_root);
    let previous_block_hash_label =
        create_label_with_title("Previous Block Hash", &previous_block_hash);
    let nonce_label = create_label_with_title("Nonce", &nonce);
    let timestamp_label = create_label_with_title("Timestamp", &timestamp);
    let version_label = create_label_with_title("Version", &version);

    block_info.add(&block_hash_label);
    block_info.add(&merkle_root_label);
    block_info.add(&previous_block_hash_label);
    block_info.add(&nonce_label);
    block_info.add(&timestamp_label);
    block_info.add(&version_label);

    block_info
}

/// Function to read the saved wallets and accounts from the file
/// Returns:
/// - Ok(Vec<AccountInfo>) if the file was read successfully
/// - Err(NodeError) if the file could not be read
pub fn read_saved_wallet_and_accounts_from_file() -> Result<Vec<AccountInfo>, NodeError> {
    let mut wallets = Vec::new();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(SAVED_ACCOUNTS)
        .map_err(|_| NodeError::FailedToOpenFile("Failed to open saved_wallet file".to_string()))?;

    if !file
        .metadata()
        .map_err(|_| NodeError::FailedToConvert("Failed to obtain file metadata".to_string()))?
        .is_file()
    {
        return Ok(wallets);
    }

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| {
            NodeError::FailedToRead(format!("Failed to read from saved wallets file: {}", e))
        })?;

        wallets.push(AccountInfo::new_from_string(line))
    }

    Ok(wallets)
}
/// Function to get an object by name from a builder
/// # Arguments
/// - builder: &gtk::Builder - The builder to get the object from
/// - name: &str - The name of the object to get
/// # Returns
/// - Ok(T) if the object was found
/// - Err(NodeError) if the object was not found
pub fn get_object_by_name<T>(builder: &gtk::Builder, name: &str) -> Result<T, NodeError>
where
    T: IsA<Object>,
{
    builder
        .object(name)
        .ok_or_else(|| NodeError::UIError(format!("Object by name: {} not found", name)))
}

/// Function to convert a timestamp to a date
/// # Arguments
/// - timestamp: u32 - The timestamp to convert
/// # Returns
/// - Ok(String) if the timestamp was converted successfully
/// - Err(NodeError) if the timestamp was invalid
pub fn timestamp_to_date(timestamp: u32) -> Result<String, NodeError> {
    let time = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0);
    let datetime = match time {
        Some(time) => DateTime::<Utc>::from_utc(time, Utc),
        None => return Err(NodeError::FailedToGetDate("Invalid Timestamp".to_string())),
    };
    let local_datetime = datetime.with_timezone(&Local);
    Ok(local_datetime.format("%d-%m-%Y %H:%M:%S").to_string())
}
