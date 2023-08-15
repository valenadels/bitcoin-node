use std::io::Read;

use crate::node_error::NodeError;

/// A wrapper enum for a variable integer
#[derive(Debug, PartialEq, Clone)]
pub enum CompactSize {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl CompactSize {
    /// Read a variable integer from a reader and return it as a CompactSize enum variant (U8, U16, U32, U64)
    ///
    /// # Arguments
    ///
    /// * `source` - A mutable reference to a reader
    ///
    /// # Returns
    ///
    /// A `Result` containing a `CompactSize` enum variant
    ///
    /// # Errors
    ///
    /// This function may return an error if the input is not a valid varint, or if an error occurs while reading from the reader.
    /// The specific error types are defined in the `NodeError` enum.
    pub fn read_varint<R: Read>(source: &mut R) -> Result<CompactSize, NodeError> {
        let mut buf = [0u8; 1];
        source
            .read_exact(&mut buf)
            .map_err(|_| NodeError::FailedToRead("Couldn't read from reader".to_string()))?;

        match buf[0] {
            0xFD => {
                let mut buf = [0u8; 2];
                source.read_exact(&mut buf).map_err(|_| {
                    NodeError::FailedToRead("Couldn't read from reader".to_string())
                })?;
                Ok(CompactSize::U16(u16::from_le_bytes(buf)))
            }
            0xFE => {
                let mut buf = [0u8; 4];
                source.read_exact(&mut buf).map_err(|_| {
                    NodeError::FailedToRead("Couldn't read from reader".to_string())
                })?;
                Ok(CompactSize::U32(u32::from_le_bytes(buf)))
            }
            0xFF => {
                let mut buf = [0u8; 8];
                source.read_exact(&mut buf).map_err(|_| {
                    NodeError::FailedToRead("Couldn't read from reader".to_string())
                })?;
                Ok(CompactSize::U64(u64::from_le_bytes(buf)))
            }
            n => Ok(CompactSize::U8(n)),
        }
    }

    /// Returns the value of the CompactSize as a u64.
    pub fn get_value(&self) -> u64 {
        match self {
            CompactSize::U8(n) => *n as u64,
            CompactSize::U16(n) => *n as u64,
            CompactSize::U32(n) => *n as u64,
            CompactSize::U64(n) => *n,
        }
    }

    /// Returns the type identifier of the `CompactSize` as a `u8`.
    pub fn get_type(&self) -> u8 {
        match self {
            CompactSize::U8(_) => 0,
            CompactSize::U16(_) => 0xFD,
            CompactSize::U32(_) => 0xFE,
            CompactSize::U64(_) => 0xFF,
        }
    }

    /// Converts the CompactSize to a byte representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self {
            CompactSize::U8(n) => n.to_le_bytes().to_vec(),
            CompactSize::U16(n) => {
                let mut result = Vec::new();
                result.extend(0xfd_u8.to_be_bytes());
                result.extend(n.to_le_bytes());
                result
            }
            CompactSize::U32(n) => {
                let mut result = Vec::new();
                result.extend(0xfe_u8.to_be_bytes());
                result.extend(n.to_le_bytes());
                result
            }
            CompactSize::U64(n) => {
                let mut result = Vec::new();
                result.extend(0xff_u8.to_be_bytes());
                result.extend(n.to_le_bytes());
                result
            }
        }
    }

    /// Create a new CompactSize enum variant based on the length of a byte array.
    ///
    /// # Arguments
    ///
    /// * `length` - The length of the byte array.
    ///
    /// # Returns
    ///
    /// A `CompactSize` enum variant representing the length.
    pub fn new(length: usize) -> CompactSize {
        if length < 0xFD {
            CompactSize::U8(length as u8)
        } else if length <= std::u16::MAX as usize {
            CompactSize::U16(length as u16)
        } else if length <= std::u32::MAX as usize {
            CompactSize::U32(length as u32)
        } else {
            CompactSize::U64(length as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_read_varint() {
        use super::CompactSize;
        use std::io::Cursor;

        let mut cursor = Cursor::new(vec![0xFD, 0x01, 0x00]);
        let varint = CompactSize::read_varint(&mut cursor).unwrap();
        assert_eq!(varint, CompactSize::U16(1));

        let mut cursor = Cursor::new(vec![0xFE, 0x01, 0x00, 0x00, 0x00]);
        let varint = CompactSize::read_varint(&mut cursor).unwrap();
        assert_eq!(varint, CompactSize::U32(1));

        let mut cursor = Cursor::new(vec![0xFF, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let varint = CompactSize::read_varint(&mut cursor).unwrap();
        assert_eq!(varint, CompactSize::U64(1));

        let mut cursor = Cursor::new(vec![0x01]);
        let varint = CompactSize::read_varint(&mut cursor).unwrap();
        assert_eq!(varint, CompactSize::U8(1));
    }

    #[test]
    fn test_compact_size_to_bytes() {
        use super::CompactSize;

        let varint = CompactSize::U8(1);
        assert_eq!(varint.to_bytes(), vec![0x01]);

        let varint = CompactSize::U16(1);
        assert_eq!(varint.to_bytes(), vec![0xfd_u8, 0x01, 0x00]);

        let varint = CompactSize::U32(1);
        assert_eq!(varint.to_bytes(), vec![0xfe_u8, 0x01, 0x00, 0x00, 0x00]);

        let varint = CompactSize::U64(1);
        assert_eq!(
            varint.to_bytes(),
            vec![0xff_u8, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_get_value_compact_size() {
        use super::CompactSize;

        let varint = CompactSize::U8(1);
        assert_eq!(varint.get_value(), 1);

        let varint = CompactSize::U16(1);
        assert_eq!(varint.get_value(), 1);

        let varint = CompactSize::U32(1);
        assert_eq!(varint.get_value(), 1);

        let varint = CompactSize::U64(1);
        assert_eq!(varint.get_value(), 1);
    }
}
