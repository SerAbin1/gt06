//! CRC-16/X-25 checksum used to validate GT06 packets.

pub fn checksum(_data: &[u8]) -> u16 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_login_crc_input() {
        // dataToCheck for the login fixture packet (LEN..SERIAL, 12 bytes)
        let data = [
            0x0d, 0x01, 0x03, 0x56, 0x93, 0x80, 0x35, 0x64, 0x38, 0x09, 0x00, 0x01,
        ];
        assert_eq!(checksum(&data), 0x911f);
    }

    #[test]
    fn matches_reference_response_crc_input() {
        // crcData for the login ack response (LEN, PROTO, SERIAL_HI, SERIAL_LO)
        let data = [0x05, 0x01, 0x00, 0x01];
        assert_eq!(checksum(&data), 0xd9dc);
    }

    #[test]
    fn empty_input_is_ffff_negated() {
        assert_eq!(checksum(&[]), 0x0000);
    }
}
