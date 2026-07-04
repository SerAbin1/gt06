//! Stream framing: finds `0x7878 .. 0x0d0a` packets in a byte stream that
//! may arrive in arbitrarily-sized chunks (e.g. from a TCP socket), and
//! reassembles them into parsed messages.

use crate::error::Error;
use crate::message::Message;
use crate::parse;

/// Buffers raw bytes from a GT06 connection and reassembles them into
/// parsed messages, tracking the IMEI seen on a prior login so it can be
/// backfilled onto later location/status/alarm messages on the same
/// connection.
#[derive(Debug, Default)]
pub struct Decoder {
    buf: Vec<u8>,
    imei: Option<String>,
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// The IMEI of the most recent login seen on this connection, if any.
    pub fn imei(&self) -> Option<&str> {
        self.imei.as_deref()
    }

    /// Feeds newly-received bytes into the decoder and returns any complete
    /// packets found, in order. An incomplete trailing packet is buffered
    /// and completed on a later call rather than discarded.
    pub fn push(&mut self, _data: &[u8]) -> Vec<Result<Message, Error>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    const LOGIN_HEX: &str = "78780d0103569380356438090001911f0d0a";
    const LOCATION_HEX: &str =
        "78781f1218060f0a1e2d87026bf998097afcac2d347b01940a138801e24000020c950d0a";

    #[test]
    fn parses_a_single_packet() {
        let mut decoder = Decoder::new();
        let results = decoder.push(&from_hex(LOGIN_HEX));
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], Ok(Message::Login(_))));
    }

    #[test]
    fn parses_multiple_concatenated_packets_in_order() {
        let mut stream = from_hex(LOGIN_HEX);
        stream.extend(from_hex(LOCATION_HEX));

        let mut decoder = Decoder::new();
        let results = decoder.push(&stream);
        assert_eq!(results.len(), 2);
        assert!(matches!(results[0], Ok(Message::Login(_))));
        assert!(matches!(results[1], Ok(Message::Location(_))));
    }

    #[test]
    fn reassembles_a_packet_split_across_two_pushes() {
        let pkt = from_hex(LOCATION_HEX);
        let half = pkt.len() / 2;

        let mut decoder = Decoder::new();
        let first = decoder.push(&pkt[..half]);
        assert!(first.is_empty(), "no complete packet yet");

        let second = decoder.push(&pkt[half..]);
        assert_eq!(second.len(), 1);
        assert!(matches!(second[0], Ok(Message::Location(_))));
    }

    #[test]
    fn reassembles_a_packet_split_byte_by_byte() {
        let pkt = from_hex(LOGIN_HEX);
        let mut decoder = Decoder::new();
        let mut results = Vec::new();
        for byte in &pkt {
            results.extend(decoder.push(std::slice::from_ref(byte)));
        }
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], Ok(Message::Login(_))));
    }

    #[test]
    fn skips_leading_noise_before_a_valid_start_marker() {
        let mut stream = vec![0xff, 0x00, 0x12];
        stream.extend(from_hex(LOGIN_HEX));

        let mut decoder = Decoder::new();
        let results = decoder.push(&stream);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], Ok(Message::Login(_))));
    }

    #[test]
    fn backfills_imei_from_a_prior_login_onto_later_messages() {
        let mut stream = from_hex(LOGIN_HEX);
        stream.extend(from_hex(LOCATION_HEX));

        let mut decoder = Decoder::new();
        let results = decoder.push(&stream);
        match &results[1] {
            Ok(Message::Location(loc)) => {
                assert_eq!(loc.imei.as_deref(), Some("356938035643809"));
            }
            other => panic!("expected Location, got {other:?}"),
        }
        assert_eq!(decoder.imei(), Some("356938035643809"));
    }

    #[test]
    fn reports_crc_mismatch_and_keeps_parsing_afterward() {
        let mut corrupt = from_hex(LOGIN_HEX);
        let len = corrupt.len();
        corrupt[len - 4] ^= 0xff;

        let mut stream = corrupt;
        stream.extend(from_hex(LOCATION_HEX));

        let mut decoder = Decoder::new();
        let results = decoder.push(&stream);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], Err(Error::CrcMismatch));
        assert!(matches!(results[1], Ok(Message::Location(_))));
    }

    #[test]
    fn location_before_any_login_has_no_imei() {
        let mut decoder = Decoder::new();
        let results = decoder.push(&from_hex(LOCATION_HEX));
        match &results[0] {
            Ok(Message::Location(loc)) => assert_eq!(loc.imei, None),
            other => panic!("expected Location, got {other:?}"),
        }
    }
}
