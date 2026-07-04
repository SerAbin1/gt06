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
pub fn parse_packet(data: &[u8]) -> Result<Message, Error> {
    if data.len() < MIN_PACKET_LEN {
        return Err(Error::TooShort);
    }
    if data[0..2] != START_MARKER {
        return Err(Error::MissingStartMarker);
    }
    if data[data.len() - 2..] != END_MARKER {
        return Err(Error::MissingEndMarker);
    }

    let received_crc = u16::from_be_bytes([data[data.len() - 4], data[data.len() - 3]]);
    let computed_crc = crc::checksum(&data[2..data.len() - 4]);
    if received_crc != computed_crc {
        return Err(Error::CrcMismatch);
    }

    match data[3] {
        0x01 => parse_login(data).map(Message::Login),
        0x12 => parse_standard_location(data).map(Message::Location),
        0x22 => parse_extended_location(data).map(Message::Location),
        0x13 => parse_status(data).map(Message::Status),
        0x16 => parse_alarm(data).map(Message::Alarm),
        other => Err(Error::UnknownProtocol(other)),
    }
}

fn parse_login(data: &[u8]) -> Result<Login, Error> {
    let packet_length = data[2];
    let serial_offset = match packet_length {
        0x0d => 12,
        0x11 => 16,
        _ => return Err(Error::InvalidLogin),
    };
    if data.len() < serial_offset + 2 {
        return Err(Error::InvalidLogin);
    }

    // IMEI is 8 bytes of BCD digits, decoded as a 16-digit string and then
    // normalized to 15 digits (devices pad with a leading zero nibble).
    let mut digits = String::with_capacity(16);
    for &byte in &data[4..12] {
        digits.push(char::from_digit((byte >> 4) as u32, 16).unwrap());
        digits.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap());
    }
    let imei = if let Some(stripped) = digits.strip_prefix('0') {
        stripped.to_string()
    } else {
        digits[..15].to_string()
    };

    let serial_number = u16::from_be_bytes([data[serial_offset], data[serial_offset + 1]]);

    Ok(Login {
        imei,
        serial_number,
    })
}

/// Decodes the GPS fix fields shared by standard location (`0x12`) and alarm
/// (`0x16`) packets, which use an identical layout for bytes 4..30.
fn parse_common_fix(data: &[u8]) -> Fix {
    let time = datetime::unix_timestamp(data[4], data[5], data[6], data[7], data[8], data[9]);
    let quantity = data[10];
    let lat_raw = u32::from_be_bytes([data[11], data[12], data[13], data[14]]);
    let lon_raw = u32::from_be_bytes([data[15], data[16], data[17], data[18]]);
    let speed_kmh = data[19];
    let course_raw = u16::from_be_bytes([data[20], data[21]]);
    let mcc = u16::from_be_bytes([data[22], data[23]]);
    let mnc = data[24];
    let lac = u16::from_be_bytes([data[25], data[26]]);
    let cell_id = ((data[27] as u32) << 16) | ((data[28] as u32) << 8) | data[29] as u32;

    let (latitude, longitude, course, real_time_gps, gps_positioned) =
        decode_course_and_coords(lat_raw, lon_raw, course_raw);

    Fix {
        time,
        satellites: Some((quantity & 0xf0) >> 4),
        satellites_active: Some(quantity & 0x0f),
        latitude,
        longitude,
        speed_kmh,
        course,
        real_time_gps,
        gps_positioned,
        mcc: Some(mcc),
        mnc: Some(mnc),
        lac: Some(lac),
        cell_id: Some(cell_id),
    }
}

/// Decodes the signed lat/lon (in degrees) and course status flags from the
/// raw GT06 fields. Latitude/longitude sign is carried in the course field.
fn decode_course_and_coords(
    lat_raw: u32,
    lon_raw: u32,
    course_raw: u16,
) -> (f64, f64, u16, bool, bool) {
    let real_time_gps = course_raw & 0x2000 != 0;
    let gps_positioned = course_raw & 0x1000 != 0;
    let west = course_raw & 0x0800 != 0;
    let north = course_raw & 0x0400 != 0;
    let course = course_raw & 0x03ff;

    let mut latitude = lat_raw as f64 / 60.0 / 30000.0;
    if !north {
        latitude = -latitude;
    }
    let mut longitude = lon_raw as f64 / 60.0 / 30000.0;
    if west {
        longitude = -longitude;
    }
    latitude = (latitude * 1_000_000.0).round() / 1_000_000.0;
    longitude = (longitude * 1_000_000.0).round() / 1_000_000.0;

    (latitude, longitude, course, real_time_gps, gps_positioned)
}

fn parse_standard_location(data: &[u8]) -> Result<Location, Error> {
    if data.len() < 35 {
        return Err(Error::InvalidLocation);
    }
    Ok(Location {
        imei: None,
        fix: parse_common_fix(data),
        extended: false,
        serial_number: tail_serial(data),
    })
}

/// Extended (`0x22`) location packets don't carry a definitive published
/// layout; this mirrors the reference implementation's length-based
/// heuristic. Cell/satellite fields are never available for these packets.
fn parse_extended_location(data: &[u8]) -> Result<Location, Error> {
    if data.len() < 25 {
        return Err(Error::InvalidLocation);
    }

    let time = datetime::unix_timestamp(data[4], data[5], data[6], data[7], data[8], data[9]);

    let (lat_raw, lon_raw, speed_kmh, course_raw) = if data.len() >= 35 {
        (
            u32::from_be_bytes([data[11], data[12], data[13], data[14]]),
            u32::from_be_bytes([data[15], data[16], data[17], data[18]]),
            data[19],
            u16::from_be_bytes([data[20], data[21]]),
        )
    } else {
        (
            u32::from_be_bytes([data[10], data[11], data[12], data[13]]),
            u32::from_be_bytes([data[14], data[15], data[16], data[17]]),
            data[18],
            u16::from_be_bytes([data[19], data[20]]),
        )
    };

    let (latitude, longitude, course, real_time_gps, gps_positioned) =
        decode_course_and_coords(lat_raw, lon_raw, course_raw);

    Ok(Location {
        imei: None,
        fix: Fix {
            time,
            satellites: None,
            satellites_active: None,
            latitude,
            longitude,
            speed_kmh,
            course,
            real_time_gps,
            gps_positioned,
            mcc: None,
            mnc: None,
            lac: None,
            cell_id: None,
        },
        extended: true,
        serial_number: tail_serial(data),
    })
}

fn parse_status(data: &[u8]) -> Result<Status, Error> {
    if data.len() < 15 {
        return Err(Error::InvalidStatus);
    }
    let terminal_info = data[4];
    let voltage = data[5];
    let gsm = data[6];

    Ok(Status {
        imei: None,
        flags: StatusFlags {
            defended: terminal_info & 0x01 != 0,
            ignition: terminal_info & 0x02 != 0,
            charging: terminal_info & 0x04 != 0,
            alarm: TerminalAlarm::from((terminal_info & 0x38) >> 3),
            gps_tracking: terminal_info & 0x40 != 0,
            relay_state: terminal_info & 0x80 != 0,
        },
        voltage_level: VoltageLevel::from(voltage),
        gsm_signal: GsmSignal::from(gsm),
        serial_number: tail_serial(data),
    })
}

fn parse_alarm(data: &[u8]) -> Result<Alarm, Error> {
    if data.len() < 40 {
        return Err(Error::InvalidAlarm);
    }
    let terminal_info = data[30];
    let voltage = data[31];
    let gsm = data[32];
    let alarm_type_raw = data[33];
    let language_raw = data[34];

    Ok(Alarm {
        imei: None,
        fix: parse_common_fix(data),
        terminal_info: AlarmTerminalInfo {
            activated: terminal_info & 0x01 != 0,
            acc_high: terminal_info & 0x02 != 0,
            charging: terminal_info & 0x04 != 0,
            alarm: TerminalAlarm::from((terminal_info & 0x38) >> 3),
            gps_tracking: terminal_info & 0x40 != 0,
            oil_electric_disconnected: terminal_info & 0x80 != 0,
        },
        voltage_level: VoltageLevel::from(voltage),
        gsm_signal: GsmSignal::from(gsm),
        alarm: AlarmEvent::from(alarm_type_raw),
        language: Language::from(language_raw),
        serial_number: tail_serial(data),
    })
}

/// The serial number always sits in the 2 bytes immediately before the CRC.
fn tail_serial(data: &[u8]) -> u16 {
    let len = data.len();
    u16::from_be_bytes([data[len - 6], data[len - 5]])
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
        let pkt =
            from_hex("78781f1218060f0a1e2d87026bf998097afcac2d347b01940a138801e24000020c950d0a");
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
