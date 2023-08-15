/// An enum representing the different types of messages that can be sent or received.
#[derive(Debug, PartialEq)]
pub enum MessageType {
    Version,
    Verack,
    Ping,
    Pong,
    Headers,
    Block,
    GetHeaders,
    SendHeaders,
    Addr,
    FeeFilter,
    Inv,
    NotFound,
    Tx,
    GetData,
}
