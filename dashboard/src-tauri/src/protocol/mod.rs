//! EAD-V1 wire protocol, host side. Layouts: `docs/protocol.md`.
//!
//! An independent implementation of the firmware's `lib/ead_core`: both are
//! checked against the golden vectors in `protocol/vectors/`, so a mistake has
//! to be made twice, in two languages, to reach the device.

pub mod cobs;
pub mod config;
mod reader;
#[cfg(test)]
mod tests;

use reader::Reader;

pub use config::parse_section;

pub const PROTOCOL_VERSION: u16 = 1;
use std::cmp::Ordering;

pub const SCHEMA_VERSION: u16 = 4;
pub const HEADER_SIZE: usize = 20;
pub const RAW_FRAME_SIZE: usize = 54;
/// Largest message the device will send (`docs/protocol.md` §7).
pub const MAX_MESSAGE_SIZE: usize = 2800;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("message shorter than a header")]
    TooShort,
    #[error("unsupported protocol version {0}")]
    Version(u16),
    #[error("header declares {declared} payload bytes, message carries {actual}")]
    LengthMismatch { declared: u32, actual: usize },
    #[error("{0} payload is malformed")]
    BadPayload(&'static str),
    #[error("frame failed CRC32")]
    BadCrc,
    #[error("malformed COBS frame")]
    BadCobs,
}

type Result<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    Hello = 0x01,
    ConfigGet = 0x02,
    ConfigSet = 0x03,
    SessionStart = 0x04,
    SessionStop = 0x05,
    Pause = 0x06,
    Resume = 0x07,
    RawSampleBatch = 0x08,
    EventBatch = 0x09,
    StepBatch = 0x0A,
    HapticBatch = 0x0B,
    Status = 0x0C,
    Ack = 0x0D,
    Error = 0x0E,
    RecoveryInfo = 0x0F,
    BackfillRequest = 0x10,
    BackfillData = 0x11,
    ServiceTest = 0x12,
}

impl MsgType {
    pub fn from_u8(value: u8) -> Option<Self> {
        use MsgType::*;
        Some(match value {
            0x01 => Hello,
            0x02 => ConfigGet,
            0x03 => ConfigSet,
            0x04 => SessionStart,
            0x05 => SessionStop,
            0x06 => Pause,
            0x07 => Resume,
            0x08 => RawSampleBatch,
            0x09 => EventBatch,
            0x0A => StepBatch,
            0x0B => HapticBatch,
            0x0C => Status,
            0x0D => Ack,
            0x0E => Error,
            0x0F => RecoveryInfo,
            0x10 => BackfillRequest,
            0x11 => BackfillData,
            0x12 => ServiceTest,
            _ => return None,
        })
    }

    /// Durable messages carry their own sequence number and can be backfilled.
    pub fn is_durable(self) -> bool {
        matches!(
            self,
            MsgType::RawSampleBatch | MsgType::EventBatch | MsgType::StepBatch | MsgType::HapticBatch
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    pub msg_type: u8,
    pub flags: u8,
    pub length: u32,
    pub sequence: u32,
    pub time_us: u64,
}

/// Header + payload, ready for a transport.
pub fn encode(msg_type: MsgType, sequence: u32, time_us: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
    out.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    out.push(msg_type as u8);
    out.push(0); // flags
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&sequence.to_le_bytes());
    out.extend_from_slice(&time_us.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Reads and validates a header. The buffer may hold more than one message, as
/// inside BACKFILL_DATA; use `header.length` to find the message boundary.
pub fn parse_header(buf: &[u8]) -> Result<Header> {
    if buf.len() < HEADER_SIZE {
        return Err(ProtocolError::TooShort);
    }
    let mut r = Reader::new(buf);
    let header = Header {
        version: r.u16()?,
        msg_type: r.u8()?,
        flags: r.u8()?,
        length: r.u32()?,
        sequence: r.u32()?,
        time_us: r.u64()?,
    };
    if header.version != PROTOCOL_VERSION {
        return Err(ProtocolError::Version(header.version));
    }
    Ok(header)
}

/// Parses exactly one message: the header must account for every byte given.
pub fn parse(msg: &[u8]) -> Result<(Header, &[u8])> {
    let header = parse_header(msg)?;
    let payload = &msg[HEADER_SIZE..];
    if header.length as usize != payload.len() {
        return Err(ProtocolError::LengthMismatch {
            declared: header.length,
            actual: payload.len(),
        });
    }
    Ok((header, payload))
}

// ---- HELLO -----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    pub schema: u16,
    pub device_state: u8,
    pub reset_reason: u8,
    pub boot_id: u32,
    pub mac: [u8; 6],
    pub who_foot: u8,
    pub who_shank: u8,
    pub capabilities: u8,
    pub config_sha256: [u8; 32],
    pub oldest_seq: u32,
    pub last_seq: u32,
    pub fw_version: String,
}

pub const CAP_HAPTICS_FITTED: u8 = 1 << 0;

/// Capability bits, `docs/protocol.md` §6.4.
pub const CAPABILITY_NAMES: [&str; 3] = ["haptics_fitted", "flash_storage", "psram_ring"];

impl Hello {
    pub fn haptics_fitted(&self) -> bool {
        self.capabilities & CAP_HAPTICS_FITTED != 0
    }

    pub fn capability_names(&self) -> Vec<&'static str> {
        names_for(self.capabilities as u16, &CAPABILITY_NAMES)
    }

    pub fn mac_string(&self) -> String {
        self.mac.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
    }
}

pub fn parse_hello(payload: &[u8]) -> Result<Hello> {
    let mut r = Reader::new(payload);
    let hello = Hello {
        schema: r.u16()?,
        device_state: r.u8()?,
        reset_reason: r.u8()?,
        boot_id: r.u32()?,
        mac: r.array::<6>()?,
        who_foot: r.u8()?,
        who_shank: r.u8()?,
        capabilities: r.u8()?,
        config_sha256: r.array::<32>()?,
        oldest_seq: r.u32()?,
        last_seq: r.u32()?,
        fw_version: r.str8()?,
    };
    Ok(hello)
}

/// Host HELLO: the schema this build implements.
pub fn hello_request() -> Vec<u8> {
    SCHEMA_VERSION.to_le_bytes().to_vec()
}

// ---- STATUS ----------------------------------------------------------------

pub const STATUS_PAYLOAD_SIZE: usize = 58;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Status {
    pub device_state: u8,
    pub link_flags: u8,
    pub faults: u16,
    pub frame_index: u32,
    pub frames_dropped: u32,
    pub shank_repeated: u32,
    pub i2c_errors: u32,
    pub imu_reinits: u32,
    pub oldest_seq: u32,
    pub last_seq: u32,
    pub ap_rssi_dbm: i8,
    pub ap_stations: u8,
    pub heap_free_min: u32,
    pub stack_free_acquisition: u16,
    pub stack_free_processing: u16,
    pub stack_free_usb: u16,
    pub stack_free_wifi: u16,
    /// 0 none, 1 collecting, 2 ready, 3 rejected (docs/protocol.md §5.3).
    pub calibration_state: u8,
    pub calibration_samples: u32,
    pub calibration_reject: u16,
    /// Gait state, `docs/protocol.md` §6.6.
    pub gait_state: u8,
    pub cycles_completed: u32,
}

pub fn parse_status(payload: &[u8]) -> Result<Status> {
    if payload.len() != STATUS_PAYLOAD_SIZE {
        return Err(ProtocolError::BadPayload("STATUS"));
    }
    let mut r = Reader::new(payload);
    Ok(Status {
        device_state: r.u8()?,
        link_flags: r.u8()?,
        faults: r.u16()?,
        frame_index: r.u32()?,
        frames_dropped: r.u32()?,
        shank_repeated: r.u32()?,
        i2c_errors: r.u32()?,
        imu_reinits: r.u32()?,
        oldest_seq: r.u32()?,
        last_seq: r.u32()?,
        ap_rssi_dbm: r.i8()?,
        ap_stations: r.u8()?,
        heap_free_min: r.u32()?,
        stack_free_acquisition: r.u16()?,
        stack_free_processing: r.u16()?,
        stack_free_usb: r.u16()?,
        stack_free_wifi: r.u16()?,
        calibration_state: r.u8()?,
        calibration_samples: r.u32()?,
        calibration_reject: r.u16()?,
        gait_state: r.u8()?,
        cycles_completed: r.u32()?,
    })
}

// ---- gait events and cycles (schema 3) --------------------------------------

/// Gait states, doc 05 §2 order.
pub const GAIT_STATES: [&str; 7] =
    ["init", "swing", "contact_transition", "stance", "foot_flat_zv", "pre_swing", "fault"];

pub const EVENT_RECORD_SIZE: usize = 14;
pub const CYCLE_RECORD_SIZE: usize = 132;
/// Bit 0 of a cycle's flags: it passed the temporal guards (doc 05 §4).
pub const CYCLE_VALID: u16 = 1 << 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GaitEventType {
    InitialContact,
    ToeOff,
    FootFlat,
    ZuptStart,
    ZuptEnd,
}

impl GaitEventType {
    fn from_byte(value: u8) -> Option<Self> {
        Some(match value {
            1 => GaitEventType::InitialContact,
            2 => GaitEventType::ToeOff,
            3 => GaitEventType::FootFlat,
            4 => GaitEventType::ZuptStart,
            5 => GaitEventType::ZuptEnd,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            GaitEventType::InitialContact => "initial_contact",
            GaitEventType::ToeOff => "toe_off",
            GaitEventType::FootFlat => "foot_flat",
            GaitEventType::ZuptStart => "zupt_start",
            GaitEventType::ZuptEnd => "zupt_end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct GaitEvent {
    pub event_type: GaitEventType,
    pub frame_index: u32,
    pub timestamp_us: u64,
}

pub fn parse_event_batch(payload: &[u8]) -> Result<Vec<GaitEvent>> {
    let mut r = Reader::new(payload);
    let count = r.u8()? as usize;
    let size = r.u8()? as usize;
    if size < EVENT_RECORD_SIZE || payload.len() != 2 + count * size {
        return Err(ProtocolError::BadPayload("EVENT_BATCH"));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = r.u8()?;
        r.u8()?; // reserved
        let event = GaitEvent {
            event_type: GaitEventType::from_byte(raw)
                .ok_or(ProtocolError::BadPayload("EVENT_BATCH"))?,
            frame_index: r.u32()?,
            timestamp_us: r.u64()?,
        };
        r.bytes(size - EVENT_RECORD_SIZE)?;
        out.push(event);
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct GaitCycle {
    pub start_frame: u32,
    pub end_frame: u32,
    pub start_us: u64,
    pub cycle_time_s: f32,
    pub stance_time_s: f32,
    pub swing_time_s: f32,
    pub stance_ratio: f32,
    pub swing_ratio: f32,
    pub cadence_steps_per_min: f32,
    pub peak_shank_rate_dps: f32,
    pub peak_dorsiflexion_deg: f32,
    pub contact_sagittal_deg: f32,
    pub peak_inversion_deg: f32,
    pub distance_m: f32,
    pub speed_mps: f32,
    pub zupt_quality: f32,
    pub valid: bool,
    /// Zero in a session with no reference: not agreement, but not scored.
    pub error_score: f32,
    pub confidence: f32,
    /// Sensor, event, feature completeness, reference stability, ZUPT.
    pub confidence_subscores: [f32; 5],
    pub deviations: [f32; 7],
    pub active_classes: u16,
    pub primary_class: u8,
}

pub fn parse_step_batch(payload: &[u8]) -> Result<Vec<GaitCycle>> {
    let mut r = Reader::new(payload);
    let count = r.u8()? as usize;
    let size = r.u8()? as usize;
    if size < CYCLE_RECORD_SIZE || payload.len() != 2 + count * size {
        return Err(ProtocolError::BadPayload("STEP_BATCH"));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let cycle = GaitCycle {
            start_frame: r.u32()?,
            end_frame: r.u32()?,
            start_us: r.u64()?,
            cycle_time_s: r.f32()?,
            stance_time_s: r.f32()?,
            swing_time_s: r.f32()?,
            stance_ratio: r.f32()?,
            swing_ratio: r.f32()?,
            cadence_steps_per_min: r.f32()?,
            peak_shank_rate_dps: r.f32()?,
            peak_dorsiflexion_deg: r.f32()?,
            contact_sagittal_deg: r.f32()?,
            peak_inversion_deg: r.f32()?,
            distance_m: r.f32()?,
            speed_mps: r.f32()?,
            zupt_quality: r.f32()?,
            valid: r.u16()? & CYCLE_VALID != 0,
            active_classes: r.u16()?,
            error_score: r.f32()?,
            confidence: r.f32()?,
            confidence_subscores: [r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?],
            deviations: [
                r.f32()?,
                r.f32()?,
                r.f32()?,
                r.f32()?,
                r.f32()?,
                r.f32()?,
                r.f32()?,
            ],
            primary_class: r.u8()?,
        };
        r.u8()?; // reserved
        r.u16()?; // reserved
        r.bytes(size - CYCLE_RECORD_SIZE)?;
        out.push(cycle);
    }
    Ok(out)
}

// ---- session control and calibration (schema 2) -----------------------------

pub const SESSION_START_PAYLOAD_SIZE: usize = 4;
pub const CALIBRATION_PAYLOAD_SIZE: usize = 128;
/// Session kinds, `docs/protocol.md` §6.8.
pub const SESSION_KIND_CALIBRATION: u8 = 1;
pub const SESSION_KIND_REFERENCE_CAPTURE: u8 = 2;
pub const SESSION_KIND_REFERENCE_CHECK: u8 = 3;
pub const SESSION_KIND_EVALUATION: u8 = 4;

/// Features in the order doc 06 §3 weighs them.
pub const FEATURE_NAMES: [&str; 7] = [
    "swing_dorsiflexion",
    "contact_plantarflexion",
    "inversion",
    "cycle_time",
    "stance_ratio",
    "cycle_distance",
    "shank_dynamics",
];

/// Error classes, `docs/protocol.md` §6.10.
pub const ERROR_CLASSES: [&str; 7] = [
    "none",
    "insufficient_dorsiflexion",
    "excess_plantarflexion",
    "inversion_deviation",
    "eversion_deviation",
    "timing_deviation",
    "overall_deviation",
];

pub const REFERENCE_PAYLOAD_SIZE: usize = 64;
/// Doc 12 §2: a reference rests on at least this many valid cycles.
pub const REFERENCE_MIN_CYCLES: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ReferenceFeature {
    pub median: f32,
    /// 1.4826 × MAD with a per-feature floor; never zero (doc 06 §2).
    pub spread: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ReferenceProfile {
    pub cycles: u16,
    /// Assigned by this dashboard; the device leaves it 0.
    pub version: u16,
    pub features: [ReferenceFeature; 7],
}

pub fn parse_reference(payload: &[u8]) -> Result<ReferenceProfile> {
    if payload.len() != REFERENCE_PAYLOAD_SIZE {
        return Err(ProtocolError::BadPayload("REFERENCE"));
    }
    let mut r = Reader::new(payload);
    let mut profile = ReferenceProfile { cycles: r.u16()?, version: r.u16()?, ..Default::default() };
    for feature in &mut profile.features {
        feature.median = r.f32()?;
        feature.spread = r.f32()?;
    }
    if profile.cycles < REFERENCE_MIN_CYCLES
        || profile.features.iter().any(|f| f.spread.partial_cmp(&0.0) != Some(Ordering::Greater))
    {
        // A spread of zero divides by zero downstream, and a profile under the
        // documented minimum is not a reference.
        return Err(ProtocolError::BadPayload("REFERENCE"));
    }
    Ok(profile)
}

pub fn encode_reference(profile: &ReferenceProfile) -> Vec<u8> {
    let mut out = Vec::with_capacity(REFERENCE_PAYLOAD_SIZE);
    out.extend_from_slice(&profile.cycles.to_le_bytes());
    out.extend_from_slice(&profile.version.to_le_bytes());
    for feature in &profile.features {
        out.extend_from_slice(&feature.median.to_le_bytes());
        out.extend_from_slice(&feature.spread.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

/// SESSION_START for a check or an evaluation: the kind and the profile it
/// judges against, which travels with the command rather than being assumed.
pub fn session_start_with_reference(kind: u8, profile: &ReferenceProfile) -> Vec<u8> {
    let mut out = session_start(kind, 0);
    out.extend_from_slice(&encode_reference(profile));
    out
}

/// SESSION_START payload: start a still window of `duration_ms`.
pub fn session_start(kind: u8, duration_ms: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(SESSION_START_PAYLOAD_SIZE);
    out.push(kind);
    out.push(0);
    out.extend_from_slice(&duration_ms.to_le_bytes());
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct CalibrationSensor {
    /// Mean rate while still, chip frame, deg/s: subtract from a reading.
    pub gyro_bias_dps: [f32; 3],
    /// Gravity as measured, anatomical frame, unit length.
    pub up: [f32; 3],
    /// Rotation taking `up` to anatomical +Z (w, x, y, z).
    pub alignment: [f32; 4],
    pub tilt_deg: f32,
    pub accel_magnitude_g: f32,
    pub gyro_std_dps: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct Calibration {
    pub kind: u8,
    /// Reject bits (docs/protocol.md §6.5); 0 means the record is usable.
    pub reject: u16,
    pub samples: u32,
    pub foot: CalibrationSensor,
    pub shank: CalibrationSensor,
}

impl Calibration {
    pub fn usable(&self) -> bool {
        self.reject == 0
    }
}

fn parse_calibration_sensor(r: &mut Reader) -> Result<CalibrationSensor> {
    let mut sensor = CalibrationSensor::default();
    for value in &mut sensor.gyro_bias_dps {
        *value = r.f32()?;
    }
    for value in &mut sensor.up {
        *value = r.f32()?;
    }
    for value in &mut sensor.alignment {
        *value = r.f32()?;
    }
    sensor.tilt_deg = r.f32()?;
    sensor.accel_magnitude_g = r.f32()?;
    sensor.gyro_std_dps = r.f32()?;
    r.bytes(8)?; // reserved
    Ok(sensor)
}

/// SESSION_STOP payload from the device: the calibration record.
pub fn parse_calibration(payload: &[u8]) -> Result<Calibration> {
    if payload.len() != CALIBRATION_PAYLOAD_SIZE {
        return Err(ProtocolError::BadPayload("SESSION_STOP"));
    }
    let mut r = Reader::new(payload);
    let kind = r.u8()?;
    r.u8()?; // reserved
    Ok(Calibration {
        kind,
        reject: r.u16()?,
        samples: r.u32()?,
        foot: parse_calibration_sensor(&mut r)?,
        shank: parse_calibration_sensor(&mut r)?,
    })
}

/// Why a record was rejected, for the operator rather than the log.
pub fn calibration_rejections(bits: u16) -> Vec<&'static str> {
    const REASONS: [(u16, &str); 4] = [
        (1 << 0, "too few samples"),
        (1 << 1, "the sensor moved"),
        (1 << 2, "acceleration was not 1 g"),
        (1 << 3, "gravity was not upward"),
    ];
    REASONS.iter().filter(|(bit, _)| bits & bit != 0).map(|(_, text)| *text).collect()
}

/// Device states, doc 07 §6 order.
pub const DEVICE_STATES: [&str; 9] = [
    "boot",
    "self_test",
    "calibrating",
    "reference_capture",
    "ready",
    "running",
    "paused",
    "fault",
    "recovery",
];

/// Fault bits, `docs/protocol.md` §6.2.
pub const FAULT_NAMES: [&str; 10] = [
    "foot_absent",
    "shank_absent",
    "foot_config",
    "shank_config",
    "foot_no_data_ready",
    "shank_no_data_ready",
    "no_psram",
    "foot_frozen",
    "shank_frozen",
    "acquisition_stalled",
];

/// Names of the set bits, in bit order.
pub fn names_for(bits: u16, names: &[&'static str]) -> Vec<&'static str> {
    names
        .iter()
        .enumerate()
        .filter(|(bit, _)| bits & (1 << bit) != 0)
        .map(|(_, name)| *name)
        .collect()
}

pub fn fault_names(faults: u16) -> Vec<&'static str> {
    names_for(faults, &FAULT_NAMES)
}

// ---- RAW_SAMPLE_BATCH ------------------------------------------------------

/// Frame status bits, `docs/protocol.md` §5.4.
pub const RAW_ORIENTATION_VALID: u16 = 1 << 8;

pub const RAW_STATUS_NAMES: [&str; 9] = [
    "foot_read_fail",
    "shank_read_fail",
    "shank_repeated",
    "foot_accel_saturated",
    "foot_gyro_saturated",
    "shank_accel_saturated",
    "shank_gyro_saturated",
    "foot_repeated",
    "orientation_valid",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawFrame {
    pub timestamp_us: u64,
    pub frame_index: u32,
    /// Chip-frame ADC counts: ax, ay, az, gx, gy, gz (DEC-007).
    pub foot: [i16; 6],
    pub shank: [i16; 6],
    /// Q15 quaternions w, x, y, z; identity unless RAW_ORIENTATION_VALID is set.
    pub q_foot: [i16; 4],
    pub q_shank: [i16; 4],
    pub status: u16,
}

pub fn parse_raw_batch(payload: &[u8]) -> Result<Vec<RawFrame>> {
    let mut r = Reader::new(payload);
    let count = r.u8()? as usize;
    let record_size = r.u8()? as usize;
    if record_size != RAW_FRAME_SIZE || payload.len() != 2 + count * RAW_FRAME_SIZE {
        return Err(ProtocolError::BadPayload("RAW_SAMPLE_BATCH"));
    }
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        frames.push(RawFrame {
            timestamp_us: r.u64()?,
            frame_index: r.u32()?,
            foot: r.i16_array::<6>()?,
            shank: r.i16_array::<6>()?,
            q_foot: r.i16_array::<4>()?,
            q_shank: r.i16_array::<4>()?,
            status: r.u16()?,
        });
    }
    Ok(frames)
}

// ---- ERROR -----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceError {
    pub cmd_seq: u32,
    pub cmd_type: u8,
    pub code: u16,
    pub detail: String,
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.code {
            1 => "BadFrame",
            2 => "SchemaMismatch",
            3 => "NotSupported",
            4 => "InvalidState",
            5 => "BadPayload",
            6 => "BackfillUnavailable",
            7 => "Rejected",
            _ => "Unknown",
        };
        write!(
            f,
            "device error {name} for command {} (type 0x{:02X}): {}",
            self.cmd_seq, self.cmd_type, self.detail
        )
    }
}

pub fn parse_device_error(payload: &[u8]) -> Result<DeviceError> {
    let mut r = Reader::new(payload);
    Ok(DeviceError {
        cmd_seq: r.u32()?,
        cmd_type: r.u8()?,
        code: r.u16()?,
        detail: r.str8()?,
    })
}

// ---- CONFIG_GET ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    pub format: u16,
    pub sha256: [u8; 32],
    pub section: Vec<u8>,
}

pub fn parse_config(payload: &[u8]) -> Result<DeviceConfig> {
    let mut r = Reader::new(payload);
    let format = r.u16()?;
    let sha256 = r.array::<32>()?;
    let length = r.u16()? as usize;
    let section = r.bytes(length)?.to_vec();
    Ok(DeviceConfig { format, sha256, section })
}

// ---- BACKFILL --------------------------------------------------------------

pub fn backfill_request(first_seq: u32, last_seq: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8);
    payload.extend_from_slice(&first_seq.to_le_bytes());
    payload.extend_from_slice(&last_seq.to_le_bytes());
    payload
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackfillChunk {
    pub cmd_seq: u32,
    pub first_seq: u32,
    pub last_seq: u32,
    pub more: bool,
    /// Complete messages, exactly as originally sent.
    pub messages: Vec<Vec<u8>>,
}

pub fn parse_backfill_data(payload: &[u8]) -> Result<BackfillChunk> {
    let mut r = Reader::new(payload);
    let cmd_seq = r.u32()?;
    let first_seq = r.u32()?;
    let last_seq = r.u32()?;
    let more = r.u8()? != 0;
    let mut messages = Vec::new();
    let mut rest = r.rest();
    while !rest.is_empty() {
        // Messages are concatenated, so only the header bounds each one.
        let header =
            parse_header(rest).map_err(|_| ProtocolError::BadPayload("BACKFILL_DATA"))?;
        let total = HEADER_SIZE + header.length as usize;
        if total > rest.len() {
            return Err(ProtocolError::BadPayload("BACKFILL_DATA"));
        }
        messages.push(rest[..total].to_vec());
        rest = &rest[total..];
    }
    Ok(BackfillChunk { cmd_seq, first_seq, last_seq, more, messages })
}

// ---- USB framing -----------------------------------------------------------

/// `00 | COBS(message ‖ CRC32-LE) | 00` (`docs/protocol.md` §2.2).
pub fn usb_frame(msg: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(msg.len() + 4);
    body.extend_from_slice(msg);
    body.extend_from_slice(&crc32(msg).to_le_bytes());
    let mut out = Vec::with_capacity(body.len() + 8);
    out.push(0);
    out.extend_from_slice(&cobs::encode(&body));
    out.push(0);
    out
}

/// Reassembles messages from the USB byte stream, discarding anything between
/// delimiters that is not a valid frame (boot text, partial writes).
#[derive(Default)]
pub struct UsbDecoder {
    buffer: Vec<u8>,
    rejected: u64,
}

impl UsbDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds received bytes and returns every complete, valid message.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        for &byte in bytes {
            if byte != 0 {
                if self.buffer.len() < MAX_MESSAGE_SIZE * 2 {
                    self.buffer.push(byte);
                }
                continue;
            }
            if self.buffer.is_empty() {
                continue;
            }
            let frame = std::mem::take(&mut self.buffer);
            match Self::decode_frame(&frame) {
                Ok(msg) => messages.push(msg),
                Err(_) => self.rejected += 1,
            }
        }
        messages
    }

    pub fn rejected(&self) -> u64 {
        self.rejected
    }

    fn decode_frame(frame: &[u8]) -> Result<Vec<u8>> {
        let body = cobs::decode(frame)?;
        if body.len() < HEADER_SIZE + 4 {
            return Err(ProtocolError::TooShort);
        }
        let (msg, crc_bytes) = body.split_at(body.len() - 4);
        let crc = u32::from_le_bytes(crc_bytes.try_into().expect("4 bytes"));
        if crc != crc32(msg) {
            return Err(ProtocolError::BadCrc);
        }
        parse(msg)?;
        Ok(msg.to_vec())
    }
}

/// IEEE 802.3 CRC-32, matching zlib.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}
