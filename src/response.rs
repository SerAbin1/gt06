//! Builds the fixed-size acknowledgement packet the server must send back
//! for message types that expect a response (login, status).

use crate::crc;

const START_BIT: [u8; 2] = [0x78, 0x78];
const END_BIT: [u8; 2] = [0x0d, 0x0a];

/// Builds the 10-byte ack packet for the given protocol number and serial.
pub fn build_ack(_protocol: u8, _serial: u16) -> [u8; 10] {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_ack_matches_reference() {
        assert_eq!(
            build_ack(0x01, 0x0001),
            [0x78, 0x78, 0x05, 0x01, 0x00, 0x01, 0xd9, 0xdc, 0x0d, 0x0a]
        );
    }

    #[test]
    fn status_ack_matches_reference() {
        assert_eq!(
            build_ack(0x13, 0x0003),
            [0x78, 0x78, 0x05, 0x13, 0x00, 0x03, 0xca, 0xe3, 0x0d, 0x0a]
        );
    }
}
