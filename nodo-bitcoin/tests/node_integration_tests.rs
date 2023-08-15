use core::time;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{mpsc, Arc, Mutex};
use std::{fs, io, net::TcpStream, path::Path};

use bitcoin_hashes::{sha256d, Error, Hash};
use glib::{Receiver, Sender};
use inoxidables_23c1::block_header::GENESIS_BLOCK_HEADER;
use inoxidables_23c1::channels::wallet_channel::WalletChannel;
use inoxidables_23c1::compact_size::CompactSize;
use inoxidables_23c1::constants::PATH_BLOCKS;
use inoxidables_23c1::transactions::outpoint::Outpoint;
use inoxidables_23c1::transactions::transaction::Transaction;
use inoxidables_23c1::transactions::tx_input::TxInput;
use inoxidables_23c1::transactions::tx_output::TxOutput;
use inoxidables_23c1::transactions::utxo_set::UtxoSet;
use inoxidables_23c1::ui::ui_message::UIMessage;
use inoxidables_23c1::wallet::bitcoin_address::BitcoinAddress;
use inoxidables_23c1::wallet::node_wallet_message::NodeWalletMsg;
use inoxidables_23c1::wallet::wallet_account_info::AccountInfo;
use inoxidables_23c1::wallet::wallet_impl::Wallet;
use inoxidables_23c1::{
    block_header::BlockHeader,
    config::load_app_config,
    constants::PATH_LOG,
    logger::Logger,
    node::{init_connection, initial_block_headers_download, read::obtain_ips},
    node_error::NodeError,
};

struct IntegrationTest {
    stream: TcpStream,
    test_logger: Logger,
}

use chrono::Duration;
use chrono::Utc;
use inoxidables_23c1::constants::STARTING_DATE;
use inoxidables_23c1::node::retry_failed_blocks;
use inoxidables_23c1::node_pools::block_downloader_pool::BlockDownloaderPool;
use std::net::SocketAddr;
use std::{env, thread};

fn set_up() -> Result<IntegrationTest, NodeError> {
    load_app_config(None)?;
    env::set_var(PATH_LOG, "log-integration-tests.txt");
    let logger = Logger::new()?;
    let mut ips = obtain_ips()?;

    Ok(IntegrationTest {
        stream: init_connection(&mut ips, &logger)?,
        test_logger: logger,
    })
}

#[cfg(not(feature = "exclude-test"))] //Excluded from github action
#[test]
fn test_blocks_download() -> Result<(), NodeError> {
    let mut integration_test = set_up()?;

    let dir = "blocks-download-test";
    env::set_var(PATH_BLOCKS, dir);
    let timestamp = (Utc::now() - Duration::minutes(60)).timestamp().to_string();
    env::set_var(STARTING_DATE, timestamp);
    empty_directory(dir).map_err(|e| NodeError::FailedToRead(e.to_string()))?;
    let ips = obtain_ips()?;
    let ipv4_addresses: Vec<SocketAddr> = ips.into_iter().filter(|addr| addr.is_ipv4()).collect();
    let ui_channel: (glib::Sender<UIMessage>, _) =
        glib::MainContext::channel(glib::Priority::default());

    //Create a pool of threads to download the blocks
    let mut pool = BlockDownloaderPool::new(
        ipv4_addresses.len(),
        &ipv4_addresses,
        integration_test.test_logger.clone(),
        &ui_channel.0,
    )?;

    //Download headers and queue the hashes
    let sender = match &pool.hash_sender {
        Some(sender) => sender,
        None => return Err(NodeError::FailedToRead("No hash sender".to_string())),
    };

    let header_blocks = initial_block_headers_download(
        &mut integration_test.stream,
        sender,
        &ui_channel.0,
        &integration_test.test_logger,
    )?;
    pool.close_channel();
    assert!(header_blocks.len() > 0);

    //Download blocks
    let mut result_join = pool.join()?;
    retry_failed_blocks(
        &result_join.1.unwrap(),
        &mut result_join.0,
        &integration_test.test_logger,
        &ui_channel.0,
    )?;

    assert_eq!(count_files_in_directory(dir), header_blocks.len());
    verify_all_blocks_are_downloaded(&header_blocks)
        .map_err(|e| NodeError::FailedToRead(e.to_string()))?;
    Ok(())
}

fn empty_directory(dir: &str) -> io::Result<()> {
    if fs::metadata(dir).is_ok() {
        fs::remove_dir_all(dir)?;
    }

    fs::create_dir(dir)?;
    Ok(())
}

fn count_files_in_directory(directory: &str) -> usize {
    let path = Path::new(directory);
    let mut file_count = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    file_count += 1;
                } else if entry_path.is_dir() {
                    file_count += count_files_in_directory(entry_path.to_str().unwrap());
                }
            }
        }
    }

    file_count
}

fn verify_all_blocks_are_downloaded(headers: &[BlockHeader]) -> Result<(), Error> {
    for header in headers.iter() {
        let hash = sha256d::Hash::hash(&header.to_bytes()).to_byte_array();

        let path = format!(
            "blocks-download-test/{}.bin",
            sha256d::Hash::from_slice(&hash)?
        );
        assert!(Path::new(&path).exists());
    }
    Ok(())
}

#[test]
fn test_wallet_operations() {
    let mut utxo_set = UtxoSet::new();

    utxo_set
        .update(
            &"blocks-test/0000000000000014e9428b9aa7427ec63e867030c1d77afeb1b182537e15be0a.bin"
                .to_string(),
        )
        .unwrap();

    let account_info = AccountInfo::new_from_values(
        "mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string(),
        "a".to_string(),
        "a".to_string(),
    );

    let (wallet_node_sender, wallet_node_receiver): (Sender<UIMessage>, Receiver<UIMessage>) =
        glib::MainContext::channel(glib::Priority::default());
    let mut wallet = Wallet::initialize_wallet_for_user(
        &Arc::new(Mutex::new(utxo_set.clone())),
        &account_info,
        &wallet_node_sender,
    )
    .unwrap();

    let new_account = AccountInfo::new_from_values(
        "mtEoVpBV5H8bbmNDEPwaoJHXnF1MxbkkQf".to_string(),
        "a".to_string(),
        "a".to_string(),
    );

    wallet
        .add_account(&utxo_set, new_account, &wallet_node_sender)
        .unwrap();

    assert_eq!(wallet.accounts.len(), 2);

    let balances = wallet.balances_for_user();

    assert_eq!(balances.len(), wallet.accounts.len());

    assert_eq!(balances[0], 0.02432823);

    assert_eq!(balances[1], 0.95717542);

    wallet_node_receiver.attach(None, move |_| glib::Continue(true));
}

#[test]
fn test_new_tx_wallet_node_connection() -> Result<(), NodeError> {
    let (sender, reciever): (mpsc::Sender<NodeWalletMsg>, mpsc::Receiver<NodeWalletMsg>) =
        mpsc::channel();

    let bc_addr = BitcoinAddress::from_string(&"mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string())?;
    let utxo_set = UtxoSet::new();
    let wallet_info = AccountInfo::new_from_values(
        "mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk".to_string(),
        "a".to_string(),
        "a".to_string(),
    );

    let (ui_sender, ui_receiver): (Sender<UIMessage>, Receiver<UIMessage>) =
        glib::MainContext::channel(glib::Priority::default());

    let wallet = Wallet::initialize_wallet_for_user(
        &Arc::new(Mutex::new(utxo_set.clone())),
        &wallet_info,
        &ui_sender,
    )?;
    thread::spawn(move || {
        ui_receiver.attach(None, move |_| glib::Continue(true));
    });

    let tx = build_tx(bc_addr);

    sender.send(NodeWalletMsg::NewTransaction(tx)).unwrap();
    drop(sender.clone());

    thread::spawn(move || {
        Wallet::handle_node_connection(
            Arc::new(Mutex::new(wallet)),
            WalletChannel {
                sender: sender,
                receiver: reciever,
            },
            ui_sender,
        )
        .unwrap();
    });
    thread::sleep(time::Duration::from_secs(2));

    //Este test no contiene asserts porque deberiamos obtener la salida del programa (stdout) y lo intentamos hacer pero sin usar crates externos no pudimos. Para verificar el correcto funcionamiento, correr el test aislado con el boton de Run. Se debe mostrar por pantalla (entre otras cosas) lo siguiente:
    // Transaction contains address: BitcoinAddress { address: [111, 186, 39, 249, 158, 0, 124, 127, 96, 90,    131, 5, 227, 24, 193, 171, 222, 60, 210, 32, 172, 213, 77, 91, 75] }
    // Received incoming transaction, which is not yet included in a block, for address: "mxVFsFW5N4mu1HPkxPttorvocvzeZ7KZyk"

    assert!(true);
    Ok(())
}

fn build_tx(bc_addr: BitcoinAddress) -> Transaction {
    let tx = Transaction {
        version: 2,
        tx_in_count: CompactSize::U8(2),
        tx_inputs: [
            TxInput {
                previous_output: Outpoint {
                    tx_id: [
                        65, 30, 235, 91, 152, 163, 61, 106, 203, 163, 188, 123, 196, 129, 19, 103,
                        98, 102, 156, 177, 216, 179, 96, 221, 209, 111, 179, 251, 110, 43, 162,
                        119,
                    ]
                    .to_vec(),
                    index: 1,
                },
                script_bytes: CompactSize::U8(0),
                signature_script: [].to_vec(),
                sequence: 4294967294,
            },
            TxInput {
                previous_output: Outpoint {
                    tx_id: [
                        237, 207, 145, 6, 157, 19, 60, 139, 122, 69, 175, 139, 164, 228, 91, 98,
                        38, 58, 49, 104, 220, 253, 29, 103, 175, 164, 110, 136, 117, 201, 56, 202,
                    ]
                    .to_vec(),
                    index: 1,
                },
                script_bytes: CompactSize::U8(0),
                signature_script: [].to_vec(),
                sequence: 4294967294,
            },
        ]
        .to_vec(),
        tx_out_count: CompactSize::U8(2),
        tx_outputs: [
            TxOutput {
                value: 1896968,
                pk_script_bytes: CompactSize::U8(22),
                pk_script: BitcoinAddress::to_pk_script(&bc_addr),
                tx_id: [
                    88, 87, 88, 173, 166, 124, 168, 235, 99, 226, 33, 159, 160, 201, 16, 233, 114,
                    238, 86, 79, 63, 102, 216, 26, 11, 65, 34, 251, 144, 41, 2, 167,
                ]
                .to_vec()
                .to_vec(),
                index: 0,
                block_path: "".to_owned(),
            },
            TxOutput {
                value: 228000,
                pk_script_bytes: CompactSize::U8(22),
                pk_script: [
                    0, 20, 93, 87, 106, 129, 244, 96, 231, 161, 237, 37, 79, 233, 191, 255, 7, 90,
                    179, 188, 69, 101,
                ]
                .to_vec(),
                tx_id: [
                    88, 87, 88, 173, 166, 124, 168, 235, 99, 226, 33, 159, 160, 201, 16, 233, 114,
                    238, 86, 79, 63, 102, 216, 26, 11, 65, 34, 251, 144, 41, 2, 167,
                ]
                .to_vec(),
                index: 1,
                block_path: "".to_owned(),
            },
        ]
        .to_vec(),
        lock_time: 2428442,
    };
    tx
}

#[test]
fn test_headers_file_doesnt_repeat_genesis() {
    let mut file_client = match fs::File::open("block_headers_client.bin") {
        Ok(file) => file,
        Err(_) => return,
    };
    let mut file_server = match fs::File::open("block_headers_server.bin") {
        Ok(file) => file,
        Err(_) => return,
    };

    let mut buffer = [0; 80].to_vec();
    let mut headers_client = Vec::new();
    let mut headers_server = Vec::new();

    for _ in 0..2 {
        match file_client.read_exact(&mut buffer) {
            Ok(_) => {
                let header = BlockHeader::from_bytes(&buffer).unwrap();
                headers_client.push(header);
            }
            Err(e) => println!("Error reading file: {}", e),
        }
        match file_server.read_exact(&mut buffer) {
            Ok(_) => {
                let header = BlockHeader::from_bytes(&buffer).unwrap();
                headers_server.push(header);
            }
            Err(e) => println!("Error reading file: {}", e),
        }
    }

    assert!(headers_client.len() == headers_server.len());
    assert!(headers_client[0].hash() == headers_server[0].hash());
    assert!(headers_client[1].hash() == headers_server[1].hash());
    assert!(headers_client[0].hash() == GENESIS_BLOCK_HEADER.hash());
    assert!(headers_client[1].hash() != GENESIS_BLOCK_HEADER.hash());
}

#[test]
fn test_compare_last_header() {
    let mut server_file = match fs::File::open("block_headers.bin") {
        Ok(file) => file,
        Err(_) => return,
    };
    let mut client_file = match fs::File::open("block_headers_client.bin") {
        Ok(file) => file,
        Err(_) => return,
    };

    let server_size = server_file.seek(SeekFrom::End(0)).unwrap();
    let client_size = client_file.seek(SeekFrom::End(0)).unwrap();

    let mut server_cursor = server_size - 80;
    let mut client_cursor = client_size - 80;

    let mut headers_client = Vec::new();
    let mut headers_server = Vec::new();

    for _ in 0..2 {
        server_file.seek(SeekFrom::Start(server_cursor)).unwrap();
        client_file.seek(SeekFrom::Start(client_cursor)).unwrap();

        let mut buffer_server = [0; 80];
        let mut buffer_client = [0; 80];

        match server_file.read_exact(&mut buffer_server) {
            Ok(_) => {
                let header = BlockHeader::from_bytes(&buffer_server.to_vec()).unwrap();
                println!("Header SERVER: {:?}", header);
                headers_server.push(header);
            }
            Err(e) => panic!("Error reading file: {}", e),
        }

        match client_file.read_exact(&mut buffer_client) {
            Ok(_) => {
                let header = BlockHeader::from_bytes(&buffer_client.to_vec()).unwrap();
                println!("Header CLIENT: {:?}", header);
                headers_client.push(header);
            }
            Err(e) => panic!("Error reading file: {}", e),
        }

        server_cursor -= 80;
        client_cursor -= 80;
    }

    assert!(headers_client.len() == headers_server.len());
    assert!(headers_client[0].hash() == headers_server[0].hash());
    assert!(headers_client[1].hash() == headers_server[1].hash());
    assert!(headers_client[1].hash() != headers_client[0].hash());
}
