//! Parsed GT06 message types.

/// A single parsed GT06 message.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Login(Login),
    Location(Location),
    Status(Status),
    Alarm(Alarm),
}

impl Message {
    /// Builds the ack packet the device expects in reply to this message, if
    /// any. `None` for message types the protocol doesn't require an ack for
    /// (location, alarm).
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    /// 15-digit IMEI, decoded from BCD.
    pub imei: String,
    pub serial_number: u16,
}

/// GPS fix data shared by location (`0x12`/`0x22`) and alarm (`0x16`) messages.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    /// Unix timestamp (seconds, UTC).
    pub time: i64,
    /// Total satellites in view. `None` for extended (`0x22`) packets, which
    /// don't carry this field.
    pub satellites: Option<u8>,
    /// Satellites used in the fix. `None` for extended (`0x22`) packets.
    pub satellites_active: Option<u8>,
    pub latitude: f64,
    pub longitude: f64,
    pub speed_kmh: u8,
    /// Heading in degrees (0-359).
    pub course: u16,
    pub real_time_gps: bool,
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
pub struct Location {
    /// IMEI of the device that sent this message, if a login was seen
    /// earlier on this connection.
    pub imei: Option<String>,
    pub fix: Fix,
    /// `true` if this came from an extended (`0x22`) location packet.
    pub extended: bool,
    pub serial_number: u16,
}

/// Status message (protocol `0x13`). Devices send this periodically as a
/// heartbeat; the server must ack it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub imei: Option<String>,
    pub flags: StatusFlags,
    pub voltage_level: VoltageLevel,
    pub gsm_signal: GsmSignal,
    pub serial_number: u16,
}

/// Alarm message (protocol `0x16`): a location fix plus terminal/alarm state.
#[derive(Debug, Clone, PartialEq)]
pub struct Alarm {
    pub imei: Option<String>,
    pub fix: Fix,
    pub terminal_info: AlarmTerminalInfo,
    pub voltage_level: VoltageLevel,
    pub gsm_signal: GsmSignal,
    pub alarm: AlarmEvent,
    pub language: Language,
    pub serial_number: u16,
}

/// Terminal info bits carried in a status (`0x13`) message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusFlags {
    pub defended: bool,
    pub ignition: bool,
    pub charging: bool,
    pub alarm: TerminalAlarm,
    pub gps_tracking: bool,
    pub relay_state: bool,
}

/// Terminal info bits carried in an alarm (`0x16`) message. Same byte layout
/// as [`StatusFlags`], but the protocol documentation names these bits
/// differently for alarm messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlarmTerminalInfo {
    pub activated: bool,
    pub acc_high: bool,
    pub charging: bool,
    pub alarm: TerminalAlarm,
    pub gps_tracking: bool,
    pub oil_electric_disconnected: bool,
}

/// Alarm condition encoded in terminal info bits 3-5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalAlarm {
    Normal,
    Shock,
    PowerCut,
    LowBattery,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmEvent {
    Normal,
    Sos,
    PowerCutAlarm,
    ShockAlarm,
    FenceIn,
    FenceOut,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Chinese,
    English,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoltageLevel {
    NoPower,
    ExtremelyLow,
    VeryLow,
    Low,
    Medium,
    High,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GsmSignal {
    NoSignal,
    ExtremelyWeak,
    VeryWeak,
    Good,
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
