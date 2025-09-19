//! Error types for the MiniBit wire protocol

/// Errors that can occur during frame encoding or decoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Buffer too small for the operation
    ShortBuffer,
    /// CRC32C checksum mismatch
    CrcMismatch,
    /// Invalid magic number in frame header
    InvalidMagic,
    /// Unsupported protocol version
    UnsupportedVersion,
    /// Unexpected end of frame data
    UnexpectedEof,
    /// Integer overflow in calculations
    Overflow,
    /// Conflicting flags in header
    FlagConflict,
    /// Decode invariant violated (internal consistency error)
    DecodeInvariant,
    /// Unsupported message type
    UnsupportedMsgType,
    /// Invalid varint encoding
    InvalidVarint,
}

impl Error {
    /// Returns a human-readable description of the error
    pub const fn description(&self) -> &'static str {
        match self {
            Error::ShortBuffer => "buffer too small for operation",
            Error::CrcMismatch => "CRC32C checksum verification failed",
            Error::InvalidMagic => "invalid magic number in frame header",
            Error::UnsupportedVersion => "unsupported protocol version",
            Error::UnexpectedEof => "unexpected end of frame data",
            Error::Overflow => "integer overflow in calculations",
            Error::FlagConflict => "conflicting flags in frame header",
            Error::DecodeInvariant => "decode invariant violated",
            Error::UnsupportedMsgType => "unsupported message type",
            Error::InvalidVarint => "invalid varint encoding",
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// Result type alias for MiniBit operations
pub type Result<T> = core::result::Result<T, Error>;
