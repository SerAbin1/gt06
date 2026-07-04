//! Stream framing: finds `0x7878 .. 0x0d0a` packets in a byte stream that
//! may arrive in arbitrarily-sized chunks (e.g. from a TCP socket), and
//! reassembles them into parsed messages.

use crate::error::Error;
use crate::message::Message;
use crate::parse;

const START_MARKER: [u8; 2] = [0x78, 0x78];
const END_MARKER: [u8; 2] = [0x0d, 0x0a];

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
    pub fn push(&mut self, data: &[u8]) -> Vec<Result<Message, Error>> {
        self.buf.extend_from_slice(data);

        let mut results = Vec::new();
        let mut pos = 0;

        loop {
            let Some(offset) = find_start_marker(&self.buf[pos..]) else {
                // No start marker in the remaining bytes. Keep a dangling
                // trailing 0x78 around in case it's the first half of a
                // marker split across this push and the next one.
                pos = if self.buf.last() == Some(&0x78) {
                    self.buf.len() - 1
                } else {
                    self.buf.len()
                };
                break;
            };
            let start = pos + offset;

            if start + 3 > self.buf.len() {
                // Have the start marker but not the length byte yet.
                pos = start;
                break;
            }

            let length = self.buf[start + 2] as usize;
            let total_len = length + 5;

            if start + total_len > self.buf.len() {
                // Framed but incomplete; wait for the rest to arrive rather
                // than discarding the start marker (unlike a naive port of
                // the reference parser, which drops a byte here and can
                // never recover a packet split across two reads).
                pos = start;
                break;
            }

            let packet = &self.buf[start..start + total_len];
            if packet[packet.len() - 2..] != END_MARKER {
                // Not actually a valid frame; treat the marker as
                // coincidental and resync one byte forward.
                pos = start + 1;
                continue;
            }

            match parse::parse_packet(packet) {
                Ok(mut message) => {
                    self.apply_session_state(&mut message);
                    results.push(Ok(message));
                }
                Err(err) => results.push(Err(err)),
            }
            pos = start + total_len;
        }

        self.buf.drain(..pos);
        results
    }

    fn apply_session_state(&mut self, message: &mut Message) {
        match message {
            Message::Login(login) => self.imei = Some(login.imei.clone()),
            Message::Location(location) => location.imei = self.imei.clone(),
            Message::Status(status) => status.imei = self.imei.clone(),
            Message::Alarm(alarm) => alarm.imei = self.imei.clone(),
        }
    }
}

fn find_start_marker(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|window| window == START_MARKER)
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
