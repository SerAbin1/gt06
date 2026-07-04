use std::fmt;

/// Errors that can occur while parsing a GT06 packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The buffer is too short to contain a valid packet.
    TooShort,
    /// The buffer doesn't start with the `0x7878` start marker.
    MissingStartMarker,
    /// The buffer doesn't end with the `0x0d0a` end marker at the length the
    /// packet's length byte says it should.
    MissingEndMarker,
    /// The CRC-16 in the packet doesn't match the computed checksum.
    CrcMismatch,
    /// The protocol number (byte 4) isn't one this crate knows how to parse.
    UnknownProtocol(u8),
    /// A login (`0x01`) packet had an unexpected length.
    InvalidLogin,
    /// A location (`0x12`/`0x22`) packet was too short for its protocol.
    InvalidLocation,
    /// A status (`0x13`) packet was too short.
    InvalidStatus,
    /// An alarm (`0x16`) packet was too short.
    InvalidAlarm,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TooShort => write!(f, "packet is shorter than the minimum GT06 packet size"),
            Error::MissingStartMarker => write!(f, "packet does not start with 0x7878"),
            Error::MissingEndMarker => write!(f, "packet does not end with 0x0d0a"),
            Error::CrcMismatch => write!(f, "CRC-16 checksum mismatch"),
            Error::UnknownProtocol(p) => write!(f, "unknown protocol number 0x{p:02x}"),
            Error::InvalidLogin => write!(f, "malformed login packet"),
            Error::InvalidLocation => write!(f, "malformed location packet"),
            Error::InvalidStatus => write!(f, "malformed status packet"),
            Error::InvalidAlarm => write!(f, "malformed alarm packet"),
        }
    }
}

impl std::error::Error for Error {}
