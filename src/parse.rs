//! Parsing of a single, already-framed GT06 packet (start marker through end
//! marker, CRC validated).

use crate::crc;
use crate::datetime;
use crate::error::Error;
use crate::message::*;

const START_MARKER: [u8; 2] = [0x78, 0x78];
const END_MARKER: [u8; 2] = [0x0d, 0x0a];
const MIN_PACKET_LEN: usize = 10;

/// Parses one complete, framed GT06 packet (including the `0x7878` start
/// marker and `0x0d0a` end marker) into a [`Message`].
///
/// The `imei` field on [`Location`], [`Status`] and [`Alarm`] messages is
/// always `None` here, since a standalone packet carries no connection
/// state — use [`crate::Decoder`] if you need it backfilled from an
/// earlier login.
pub fn parse_packet(_data: &[u8]) -> Result<Message, Error> {
    todo!()
}

fn parse_login(_data: &[u8]) -> Result<Login, Error> {
    todo!()
}

fn parse_common_fix(_data: &[u8]) -> Fix {
    todo!()
}

fn parse_standard_location(_data: &[u8]) -> Result<Location, Error> {
    todo!()
}

fn parse_extended_location(_data: &[u8]) -> Result<Location, Error> {
    todo!()
}

fn parse_status(_data: &[u8]) -> Result<Status, Error> {
    todo!()
}

fn parse_alarm(_data: &[u8]) -> Result<Alarm, Error> {
    todo!()
}

fn tail_serial(_data: &[u8]) -> u16 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn parses_login() {
        let pkt = from_hex("78780d0103569380356438090001911f0d0a");
        let msg = parse_packet(&pkt).unwrap();
        match msg {
            Message::Login(login) => {
                assert_eq!(login.imei, "356938035643809");
                assert_eq!(login.serial_number, 1);
            }
            other => panic!("expected Login, got {other:?}"),
        }
    }

    #[test]
    fn parses_standard_location() {
        let pkt = from_hex(
            "78781f1218060f0a1e2d87026bf998097afcac2d347b01940a138801e24000020c950d0a",
        );
        let msg = parse_packet(&pkt).unwrap();
        match msg {
            Message::Location(loc) => {
                assert!(!loc.extended);
                assert_eq!(loc.imei, None);
                assert_eq!(loc.serial_number, 2);
                assert_eq!(loc.fix.time, 1_718_447_445);
                assert_eq!(loc.fix.satellites, Some(8));
                assert_eq!(loc.fix.satellites_active, Some(7));
                assert!((loc.fix.latitude - 22.5726).abs() < 1e-6);
                assert!((loc.fix.longitude - 88.3639).abs() < 1e-6);
                assert_eq!(loc.fix.speed_kmh, 45);
                assert_eq!(loc.fix.course, 123);
                assert!(loc.fix.real_time_gps);
                assert!(loc.fix.gps_positioned);
                assert_eq!(loc.fix.mcc, Some(404));
                assert_eq!(loc.fix.mnc, Some(10));
                assert_eq!(loc.fix.lac, Some(5000));
                assert_eq!(loc.fix.cell_id, Some(123456));
            }
            other => panic!("expected Location, got {other:?}"),
        }
    }

    #[test]
    fn parses_status() {
        let pkt = from_hex("78780a13c70403000000033d870d0a");
        let msg = parse_packet(&pkt).unwrap();
        match msg {
            Message::Status(status) => {
                assert_eq!(status.imei, None);
                assert_eq!(status.serial_number, 3);
                assert_eq!(
                    status.flags,
                    StatusFlags {
                        defended: true,
                        ignition: true,
                        charging: true,
                        alarm: TerminalAlarm::Normal,
                        gps_tracking: true,
                        relay_state: true,
                    }
                );
                assert_eq!(status.voltage_level, VoltageLevel::Medium);
                assert_eq!(status.gsm_signal, GsmSignal::Good);
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn parses_alarm() {
        let pkt = from_hex(
            "7878241618060f0a1e2d87026bf998097afcac2d347b01940a138801e240c7040302010004abfe0d0a",
        );
        let msg = parse_packet(&pkt).unwrap();
        match msg {
            Message::Alarm(alarm) => {
                assert_eq!(alarm.imei, None);
                assert_eq!(alarm.serial_number, 4);
                assert_eq!(alarm.fix.time, 1_718_447_445);
                assert!((alarm.fix.latitude - 22.5726).abs() < 1e-6);
                assert!((alarm.fix.longitude - 88.3639).abs() < 1e-6);
                assert_eq!(
                    alarm.terminal_info,
                    AlarmTerminalInfo {
                        activated: true,
                        acc_high: true,
                        charging: true,
                        alarm: TerminalAlarm::Normal,
                        gps_tracking: true,
                        oil_electric_disconnected: true,
                    }
                );
                assert_eq!(alarm.voltage_level, VoltageLevel::Medium);
                assert_eq!(alarm.gsm_signal, GsmSignal::Good);
                assert_eq!(alarm.alarm, AlarmEvent::PowerCutAlarm);
                assert_eq!(alarm.language, Language::Chinese);
            }
            other => panic!("expected Alarm, got {other:?}"),
        }
    }

    #[test]
    fn rejects_crc_mismatch() {
        let mut pkt = from_hex("78780d0103569380356438090001911f0d0a");
        let len = pkt.len();
        pkt[len - 4] ^= 0xff; // corrupt CRC high byte
        assert_eq!(parse_packet(&pkt), Err(Error::CrcMismatch));
    }

    #[test]
    fn rejects_unknown_protocol() {
        let mut pkt = from_hex("78780d0103569380356438090001911f0d0a");
        pkt[3] = 0x99;
        // CRC now also won't match since it covers the protocol byte, but
        // unknown protocol should be reported rather than a CRC error —
        // recompute a matching CRC so this test isolates protocol dispatch.
        let crc = crc::checksum(&pkt[2..pkt.len() - 4]);
        let len = pkt.len();
        pkt[len - 4] = (crc >> 8) as u8;
        pkt[len - 3] = (crc & 0xff) as u8;
        assert_eq!(parse_packet(&pkt), Err(Error::UnknownProtocol(0x99)));
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(parse_packet(&[0x78, 0x78, 0x01]), Err(Error::TooShort));
    }

    #[test]
    fn rejects_missing_start_marker() {
        let mut pkt = from_hex("78780d0103569380356438090001911f0d0a");
        pkt[0] = 0x00;
        assert_eq!(parse_packet(&pkt), Err(Error::MissingStartMarker));
    }

    #[test]
    fn rejects_missing_end_marker() {
        let mut pkt = from_hex("78780d0103569380356438090001911f0d0a");
        let len = pkt.len();
        pkt[len - 1] = 0x00;
        assert_eq!(parse_packet(&pkt), Err(Error::MissingEndMarker));
    }
}
