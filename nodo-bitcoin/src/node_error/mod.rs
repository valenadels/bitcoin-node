/// Enum representing possible errors that can occur while running the node.
#[derive(Debug)]
pub enum NodeError {
    /// Failed to connect to a node with the given IP.
    FailedToConnect(String),
    /// Failed to flush data to a stream.
    FailedToFlush(String),
    /// Failed to send a message through a stream.
    FailedToSendMessage(String),
    /// Failed to receive a message from a stream.
    FailedToReceiveMessage(String),
    /// Failed to read an exact amount of bytes from a stream.
    FailedToReadExact(String),
    /// Failed to read data from a stream.
    FailedToRead(String),
    /// Failed to write data.
    FailedToWrite(String),
    /// Failed to write all the data to a stream.
    FailedToWriteAll(String),
    /// Failed to parse data into a certain type.
    FailedToParse(String),
    /// Failed to load the application configuration.
    FailedToLoadConfig(String),
    /// Failed to create a version message for a node.
    FailedToCreateVersionMessage(String),
    /// Failed to create the byte representation of a version message.
    FailedToCreateVersionMessageBytes(String),
    /// Failed to create the header field for a version message.
    FailedToCreateVersionMessageHeader(String),
    /// Failed to create the payload for a version message.
    FailedToCreateVersionMessagePayload(String),
    /// Failed to calculate the checksum for a version message.
    FailedToCreateVersionMessageChecksum(String),
    /// Failed to open a file.
    FailedToOpenFile(String),
    /// Failed to create a header field for a certain message type.
    FailedToCreateHeaderField(String),
    /// The required environment variable was not found.
    EnvironVarNotFound(String),
    /// Failed to connect to a node using DNS resolution.
    FailedToConnectDNS(String),
    /// Failed to convert data between types.
    FailedToConvert(String),
    ///The header of a block has an invalid length.
    InvalidBlockHeaderLength(String),
    /// The header of a block has an invalid field.
    InvalidBlockHeaderField(String),
    /// Failed to create a getheaders message.
    FailedToCreateGetheadersMessage(String),
    /// Invalid size of prefix.
    InvalidSizeOfPrefix(String),
    /// Invalid size of headers.
    InvalidSizeOfHeaders(String),
    /// Failed to convert bytes to string.
    FailedToConvertToString(String),
    /// Failed to read from the stream.
    ReadTimeoutFromStream(String),
    /// No ips found.
    NoIpsFound(String),
    /// Failed to determine command type.
    CommandTypeError(String),
    /// Invalid size of pool, it must be greater than 0.
    InvalidSizeOfPool(String),
    /// Builder thread could not be created.
    FailedToCreateThread(String),
    /// Size of the field is invalid.
    InvalidSizeOfField(String),
    ///Failed to obtain stream.
    MutexError(String),
    /// Invalid message format.
    InvalidMessageFormat(String),
    /// Failed to get ip.
    FailedToGetIp(String),
    /// Failed to clone stream.
    FailedToGetStream(String),
    /// Failed to download block.
    FailedToDownloadBlock(String),
    /// Failed to download block header.
    FailedToDownloadBlockHeader(String),
    /// Failed to send job to thread pool.
    FailedToSendJobToThreadPool(String),
    /// Failed to send hash through channel.
    FailedToSendHash(String),
    /// Failed to join thread in thread pool.
    FailedToJoinThread(String),
    /// Invalid Merkle root.
    InvalidMerkleRoot(String),
    /// Invalid hash.
    InvalidProofOfWork(String),
    /// Failed to get output from transaction.
    FailedToCreateOutpoint(String),
    /// Failed to get input from transaction.
    FailedToCreateTxInput(String),
    /// Failed to get output transaction from block.
    FailedToCreateTxOutput(String),
    /// Failed to get coinbase transaction from block.
    FailedToCreateCoinbaseTransaction(String),
    /// Invalid nBits.
    InvalidNBits(String),
    ///Error in the interface.
    UIError(String),
    /// Not enough coins in wallet.
    NotEnoughCoins(String),
    /// Sync node does not have the block requested.
    SyncNodeDoesNotHaveTheBlock(String),
    /// Error in utxo set mutex in thread pool.
    UtxoSetMutexError(String),
    //Error related to the logger.
    FailedToLog(String),
    /// Failed to sign transaction.
    SigningError(String),
    /// Error in wallet mutex in thread pool.
    WalletMutexError(String),
    /// The PK Script received is not a P2PKH script.
    NotP2PKHScript(String),
    /// Account not found.
    AccountNotFound(String),
    /// Node Sender error.
    NodeSenderError(String),
    /// Invalid hexadecimal string.
    InvalidHexString(String),
    /// Invalid merkle tree.
    InvalidMerkleTree(String),
    /// Failed to create transaction.
    FailedToCreateTransaction(String),
    /// Failed to create wallet from user data.
    FailedToCreateWallet(String),
    /// Failed to get the first account in wallet.
    FailedToObtainAccount(String),
    /// Failed to change the current account.
    FailedToChangeAccount(String),
    /// Failed to get date from timestamp
    FailedToGetDate(String),
    /// Failed to lock arc mutex wallet
    FailedToLockWallet(String),
    ///Failed to bind.
    FailedToBind(String),
    /// Starting header not found.
    StartingHeaderNotFound(String),
    /// Invalid type.
    InvalidType(String),
    /// Tcp stream connection was closed.
    TcpStreamNotConnected(String),
    /// The verack was not received properly.
    HandshakeFailed(String),
    /// Failed to clone stream
    FailedToCloneStream(String),
    ///Failed to delete file
    FailedToDeleteFile(String),
}
