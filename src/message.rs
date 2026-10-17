//! Parsed GT06 message types.

/// A single parsed GT06 message.
///
/// Non-exhaustive: new variants will be added as more protocol numbers are
/// supported.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Message {
    /// Login (`0x01`).
    Login(Login),
    /// Location (`0x12`) or extended location (`0x22`).
    Location(Location),
    /// Status / heartbeat (`0x13`).
    Status(Status),
    /// Alarm (`0x16`).
    Alarm(Alarm),
}

impl Message {
    /// Builds the ack packet the device expects in reply to this message, if
    /// any. `None` for message types the protocol doesn't require an ack for
    /// (location, alarm).
    ///
    /// ```
    /// # let packet = [
    /// #     0x78, 0x78, 0x0d, 0x01, 0x03, 0x56, 0x93, 0x80, 0x35, 0x64, 0x38, 0x09,
    /// #     0x00, 0x01, 0x91, 0x1f, 0x0d, 0x0a,
    /// # ];
    /// let login = gt06::parse_packet(&packet)?;
    /// assert_eq!(
    ///     login.ack_bytes(),
    ///     Some([0x78, 0x78, 0x05, 0x01, 0x00, 0x01, 0xd9, 0xdc, 0x0d, 0x0a]),
    /// );
    /// # Ok::<(), gt06::Error>(())
    /// ```
    pub fn ack_bytes(&self) -> Option<[u8; 10]> {
        match self {
            Message::Login(login) => Some(crate::response::build_ack(0x01, login.serial_number)),
            Message::Status(status) => Some(crate::response::build_ack(0x13, status.serial_number)),
            Message::Location(_) | Message::Alarm(_) => None,
        }
    }

    /// `true` if this message expects an ack reply. Equivalent to
    /// `self.ack_bytes().is_some()`.
    pub fn expects_ack(&self) -> bool {
        self.ack_bytes().is_some()
    }
}

/// Login message (protocol `0x01`). The device sends this once per
/// connection to identify itself; the server must ack it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Login {
    /// 15-digit IMEI, decoded from BCD.
    pub imei: String,
    /// Packet serial number, echoed back in the ack.
    pub serial_number: u16,
}

/// GPS fix data shared by location (`0x12`/`0x22`) and alarm (`0x16`) messages.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Fix {
    /// Unix timestamp (seconds, UTC).
    pub time: i64,
    /// Total satellites in view. `None` for extended (`0x22`) packets, which
    /// don't carry this field.
    pub satellites: Option<u8>,
    /// Satellites used in the fix. `None` for extended (`0x22`) packets.
    pub satellites_active: Option<u8>,
    /// Latitude in signed decimal degrees (negative is south), rounded to 6
    /// decimal places.
    pub latitude: f64,
    /// Longitude in signed decimal degrees (negative is west), rounded to 6
    /// decimal places.
    pub longitude: f64,
    /// Ground speed in km/h.
    pub speed_kmh: u8,
    /// Heading in degrees (0-359).
    pub course: u16,
    /// `true` if this is a real-time fix, `false` if it's a buffered
    /// (differential) one.
    pub real_time_gps: bool,
    /// `true` if the GPS had a position lock when this fix was taken.
    pub gps_positioned: bool,
    /// Mobile country code. `None` for extended (`0x22`) packets.
    pub mcc: Option<u16>,
    /// Mobile network code. `None` for extended (`0x22`) packets.
    pub mnc: Option<u8>,
    /// Location area code. `None` for extended (`0x22`) packets.
    pub lac: Option<u16>,
    /// Cell tower ID. `None` for extended (`0x22`) packets.
    pub cell_id: Option<u32>,
}

/// Location message (protocol `0x12`, or `0x22` when [`Location::extended`] is set).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Location {
    /// IMEI of the device that sent this message, if a login was seen
    /// earlier on this connection.
    pub imei: Option<String>,
    /// The reported GPS fix.
    pub fix: Fix,
    /// `true` if this came from an extended (`0x22`) location packet.
    pub extended: bool,
    /// Packet serial number.
    pub serial_number: u16,
}

/// Status message (protocol `0x13`). Devices send this periodically as a
/// heartbeat; the server must ack it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Status {
    /// IMEI of the device that sent this message, if a login was seen
    /// earlier on this connection.
    pub imei: Option<String>,
    /// Terminal info bits.
    pub flags: StatusFlags,
    /// Battery voltage level.
    pub voltage_level: VoltageLevel,
    /// GSM signal strength.
    pub gsm_signal: GsmSignal,
    /// Packet serial number, echoed back in the ack.
    pub serial_number: u16,
}

/// Alarm message (protocol `0x16`): a location fix plus terminal/alarm state.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Alarm {
    /// IMEI of the device that sent this message, if a login was seen
    /// earlier on this connection.
    pub imei: Option<String>,
    /// The GPS fix at the time of the alarm.
    pub fix: Fix,
    /// Terminal info bits.
    pub terminal_info: AlarmTerminalInfo,
    /// Battery voltage level.
    pub voltage_level: VoltageLevel,
    /// GSM signal strength.
    pub gsm_signal: GsmSignal,
    /// The event that triggered this alarm.
    pub alarm: AlarmEvent,
    /// Language the device expects any reply text in.
    pub language: Language,
    /// Packet serial number.
    pub serial_number: u16,
}

/// Terminal info bits carried in a status (`0x13`) message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusFlags {
    /// Bit 0: defence (arming) is activated.
    pub defended: bool,
    /// Bit 1: ACC (ignition) is high.
    pub ignition: bool,
    /// Bit 2: the device is charging.
    pub charging: bool,
    /// Bits 3-5: the current alarm condition.
    pub alarm: TerminalAlarm,
    /// Bit 6: GPS tracking is on.
    pub gps_tracking: bool,
    /// Bit 7: the oil/electricity relay is disconnected.
    pub relay_state: bool,
}

/// Terminal info bits carried in an alarm (`0x16`) message. Same byte layout
/// as [`StatusFlags`], but the protocol documentation names these bits
/// differently for alarm messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlarmTerminalInfo {
    /// Bit 0: defence (arming) is activated.
    pub activated: bool,
    /// Bit 1: ACC (ignition) is high.
    pub acc_high: bool,
    /// Bit 2: the device is charging.
    pub charging: bool,
    /// Bits 3-5: the current alarm condition.
    pub alarm: TerminalAlarm,
    /// Bit 6: GPS tracking is on.
    pub gps_tracking: bool,
    /// Bit 7: the oil/electricity relay is disconnected.
    pub oil_electric_disconnected: bool,
}

/// Alarm condition encoded in terminal info bits 3-5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalAlarm {
    /// No alarm.
    Normal,
    /// Shock (vibration) alarm.
    Shock,
    /// External power cut.
    PowerCut,
    /// Low battery.
    LowBattery,
    /// SOS button pressed.
    Sos,
}

impl From<u8> for TerminalAlarm {
    fn from(bits: u8) -> Self {
        match bits {
            1 => TerminalAlarm::Shock,
            2 => TerminalAlarm::PowerCut,
            3 => TerminalAlarm::LowBattery,
            4 => TerminalAlarm::Sos,
            _ => TerminalAlarm::Normal,
        }
    }
}

/// Top-level alarm event carried in an alarm (`0x16`) message. Distinct from
/// [`TerminalAlarm`], which is a different bitfield in the same message.
///
/// Non-exhaustive: alarm codes currently reported as [`AlarmEvent::Unknown`]
/// may get their own variant in a future release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlarmEvent {
    /// No alarm (`0x00`).
    Normal,
    /// SOS button pressed (`0x01`).
    Sos,
    /// External power cut (`0x02`).
    PowerCutAlarm,
    /// Shock (vibration) alarm (`0x03`).
    ShockAlarm,
    /// Entered a geofence (`0x04`).
    FenceIn,
    /// Left a geofence (`0x05`).
    FenceOut,
    /// An alarm code this crate doesn't recognize.
    Unknown(u8),
}

impl From<u8> for AlarmEvent {
    fn from(value: u8) -> Self {
        match value {
            0x00 => AlarmEvent::Normal,
            0x01 => AlarmEvent::Sos,
            0x02 => AlarmEvent::PowerCutAlarm,
            0x03 => AlarmEvent::ShockAlarm,
            0x04 => AlarmEvent::FenceIn,
            0x05 => AlarmEvent::FenceOut,
            other => AlarmEvent::Unknown(other),
        }
    }
}

/// Language the device expects reply text in, carried in an alarm (`0x16`)
/// message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// Chinese (`0x01`).
    Chinese,
    /// English (`0x02`).
    English,
    /// A language code this crate doesn't recognize.
    Unknown(u8),
}

impl From<u8> for Language {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Language::Chinese,
            0x02 => Language::English,
            other => Language::Unknown(other),
        }
    }
}

/// Battery voltage level, from lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoltageLevel {
    /// No power (shutdown), `0`.
    NoPower,
    /// Extremely low battery, `1`.
    ExtremelyLow,
    /// Very low battery, `2`.
    VeryLow,
    /// Low battery, `3`.
    Low,
    /// Medium, `4`.
    Medium,
    /// High, `5`.
    High,
    /// Very high, `6`.
    VeryHigh,
}

impl From<u8> for VoltageLevel {
    fn from(value: u8) -> Self {
        match value {
            1 => VoltageLevel::ExtremelyLow,
            2 => VoltageLevel::VeryLow,
            3 => VoltageLevel::Low,
            4 => VoltageLevel::Medium,
            5 => VoltageLevel::High,
            6 => VoltageLevel::VeryHigh,
            _ => VoltageLevel::NoPower,
        }
    }
}

/// GSM signal strength, from weakest to strongest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GsmSignal {
    /// No signal, `0`.
    NoSignal,
    /// Extremely weak signal, `1`.
    ExtremelyWeak,
    /// Very weak signal, `2`.
    VeryWeak,
    /// Good signal, `3`.
    Good,
    /// Strong signal, `4`.
    Strong,
}

impl From<u8> for GsmSignal {
    fn from(value: u8) -> Self {
        match value {
            1 => GsmSignal::ExtremelyWeak,
            2 => GsmSignal::VeryWeak,
            3 => GsmSignal::Good,
            4 => GsmSignal::Strong,
            _ => GsmSignal::NoSignal,
        }
    }
}
