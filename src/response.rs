//! Builds the fixed-size acknowledgement packet the server must send back
//! for message types that expect a response (login, status).

use crate::crc;

const START_BIT: [u8; 2] = [0x78, 0x78];
const END_BIT: [u8; 2] = [0x0d, 0x0a];

/// Builds the 10-byte ack packet for the given protocol number and serial.
pub fn build_ack(protocol: u8, serial: u16) -> [u8; 10] {
    let serial_bytes = serial.to_be_bytes();
    let crc_input = [0x05, protocol, serial_bytes[0], serial_bytes[1]];
    let crc = crc::checksum(&crc_input).to_be_bytes();

    let mut packet = [0u8; 10];
    packet[0..2].copy_from_slice(&START_BIT);
    packet[2] = 0x05;
    packet[3] = protocol;
    packet[4..6].copy_from_slice(&serial_bytes);
    packet[6..8].copy_from_slice(&crc);
    packet[8..10].copy_from_slice(&END_BIT);
    packet
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
