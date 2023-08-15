use crate::constants::LENGTH_VERACK_MESSAGE;

/// The VERACK_MESSAGE constant is used to send a Verack message in the Bitcoin network.
///  It always has the same format, with the 'magic' field identifying the network of origin,
/// the 'command' field indicating the type of message (always "verack" in this case),
/// the 'payload size' field being 0, and the 'checksum' field being a hash computed using sha256
///  (which will always be 0x5DF6E0E2 since the payload is 0).
pub const VERACK_MESSAGE: [u8; 24] = [
    0x0b, 0x11, 0x09, 0x07, // magic
    0x76, 0x65, 0x72, 0x61, 0x63, 0x6b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // command
    0x00, 0x00, 0x00, 0x00, // payload size
    0x5d, 0xf6, 0xe0, 0xe2, // checksum
];

/// Returns the verack message, which is a constant message sent to acknowledge a received message.
pub fn get_verack_message() -> [u8; LENGTH_VERACK_MESSAGE] {
    VERACK_MESSAGE
}

/// Check whether a received message is a verack message.
///
/// # Arguments
///
/// * verack_recibido - A slice containing the received message.
///
/// # Returns
///
/// A boolean value that indicates whether the message is a verack message or not.
/// true indicates that it is, while false indicates that it is not.
pub fn is_verack_message(received_verack: &[u8]) -> bool {
    if received_verack.len() < LENGTH_VERACK_MESSAGE {
        return false;
    }

    received_verack[0..LENGTH_VERACK_MESSAGE] == VERACK_MESSAGE[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_verack_message_recibe_un_verack_y_devuelve_true() {
        let verack_serializado = VERACK_MESSAGE;
        assert!(is_verack_message(&verack_serializado));
    }
}
