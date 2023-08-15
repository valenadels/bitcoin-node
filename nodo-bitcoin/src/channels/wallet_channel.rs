use std::sync::mpsc::{self, Receiver, Sender};

use crate::{node_error::NodeError, wallet::node_wallet_message::NodeWalletMsg};

/// Channel for communication between the node and the wallet
pub struct WalletChannel {
    /// Sender for the channel
    pub sender: Sender<NodeWalletMsg>,
    /// Receiver for the channel
    pub receiver: Receiver<NodeWalletMsg>,
}

impl WalletChannel {
    /// Create a new channel for communication between the node and the wallet
    pub fn new() -> Self {
        let (wallet_sender, wallet_receiver) = mpsc::channel();

        WalletChannel {
            sender: wallet_sender,
            receiver: wallet_receiver,
        }
    }
    /// Create a pair of channels for communication between the node and the wallet
    pub fn create_pairs() -> (WalletChannel, WalletChannel) {
        let wallet_node_channel = (WalletChannel::new(), WalletChannel::new());
        let node_channel = WalletChannel {
            sender: wallet_node_channel.0.sender,
            receiver: wallet_node_channel.1.receiver,
        };
        let wallet_channel = WalletChannel {
            sender: wallet_node_channel.1.sender,
            receiver: wallet_node_channel.0.receiver,
        };
        (wallet_channel, node_channel)
    }
    /// Send a message to the wallet
    pub fn send(&self, message: NodeWalletMsg) -> Result<(), NodeError> {
        self.sender.send(message).map_err(|e| {
            NodeError::FailedToSendMessage(format!(
                "Failed to send message to wallet channel {}",
                e
            ))
        })
    }
    /// Receive a message from the wallet
    pub fn receive(&self) -> Result<NodeWalletMsg, NodeError> {
        self.receiver.recv().map_err(|_| {
            NodeError::FailedToReceiveMessage(
                "Failed to receive message from wallet channel".to_string(),
            )
        })
    }
}

impl Default for WalletChannel {
    /// Create a new channel for communication between the node and the wallet
    fn default() -> Self {
        Self::new()
    }
}
