use glib::{Receiver, Sender};

use crate::{node_error::NodeError, ui::ui_message::UIMessage};

/// Represents a communication channel between the user interface (UI) component and other components.
///
/// The `UiChannel` consists of a `sender` and a `receiver` that are used to send and receive messages between components.
pub struct UiChannel {
    /// The sender of the channel used to send messages from the UI component to other components.
    pub sender: Sender<UIMessage>,
    /// The receiver of the channel used to receive messages from other components to the UI component.
    pub receiver: Receiver<UIMessage>,
}

impl UiChannel {
    /// Creates a new `UiChannel` instance.
    ///
    /// The `UiChannel` facilitates communication between the UI thread and other threads by using a sender-receiver mechanism.
    pub fn new() -> Self {
        let (ui_sender, ui_receiver) = glib::MainContext::channel(glib::Priority::default());

        UiChannel {
            sender: ui_sender,
            receiver: ui_receiver,
        }
    }
    /// Sends a `UIMessage` to the UI channel.
    ///
    /// This function is used to send messages from other threads to the UI thread through the `UiChannel`.
    ///
    /// # Arguments
    ///
    /// * `message` - The `UIMessage` to be sent to the UI channel.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the message was successfully sent, or an `Err` variant containing a `NodeError` if the message sending failed.
    ///
    /// # Errors
    ///
    /// This function can return an `Err` variant with a `NodeError` in the following cases:
    ///
    /// * The message sending failed due to an internal error.
    pub fn send(&self, message: UIMessage) -> Result<(), NodeError> {
        self.sender.send(message).map_err(|_| {
            NodeError::FailedToSendMessage("Failed to send message to UI channel".to_string())
        })
    }
    /// Returns the sender of the channel.
    pub fn sender(&self) -> Sender<UIMessage> {
        self.sender.clone()
    }
}

impl Default for UiChannel {
    /// Creates a new `UiChannel` with a default sender and receiver.
    fn default() -> Self {
        Self::new()
    }
}
