//! The device configuration section (`docs/protocol.md` §5.5, format 1).
//!
//! This is the device's own copy of the fixed V1 values, hashed into the
//! configuration hash recorded with every session (doc 18). The host needs the
//! scale factors and mount maps to show anatomical units, and the export needs
//! the whole thing for `metadata.json` (doc 10 §6).

use super::{ProtocolError, Reader, Result};

/// 3×3 signed permutation, row-major: `anatomical = m · chip` (DEC-009).
pub type MountMap = [[i8; 3]; 3];

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ImuConfig {
    pub foot_address: u8,
    pub shank_address: u8,
    pub i2c_hz: u32,
    pub sample_hz: u16,
    pub accel_range_g: u8,
    pub gyro_range_dps: u16,
    pub dlpf_hz: u8,
    pub dlpf_cfg: u8,
    pub sample_rate_divider: u8,
    pub accel_lsb_per_g: f32,
    pub gyro_lsb_per_dps: f32,
    pub foot_mount: MountMap,
    pub shank_mount: MountMap,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PinConfig {
    pub i2c_sda: u8,
    pub i2c_scl: u8,
    pub foot_imu_int: u8,
    pub shank_imu_int: u8,
    pub motors: [u8; 6],
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GaitConfig {
    pub gait_lowpass_hz: f32,
    pub event_lowpass_hz: f32,
    pub min_cycle_s: f32,
    pub max_cycle_s: f32,
    pub contact_guard_ms: u16,
    pub toeoff_guard_ms: u16,
    pub ic_window_ms: u16,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ZuptConfig {
    pub accel_tolerance_g: f32,
    pub gyro_threshold_dps: f32,
    pub min_duration_ms: u16,
    pub entry_hysteresis_ms: u16,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ErrorConfig {
    pub class_activation: f32,
    pub haptic_on: f32,
    pub haptic_off: f32,
    pub confidence_haptic: f32,
    pub robust_z_clip: f32,
    /// Doc 06 §3 order: swing dorsiflexion, IC plantarflexion, inversion/eversion,
    /// timing, stance/swing ratio, cycle distance, shank dynamics.
    pub weights: [f32; 7],
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HapticConfig {
    /// False in this build: no ERM driver channels are fitted (DEC-006).
    pub fitted: bool,
    pub pwm_hz: u16,
    pub pwm_bits: u8,
    pub min_duty: u8,
    pub max_duty: u8,
    pub intensity_exponent: f32,
    pub max_on_s: u8,
    pub rolling_window_s: u8,
    pub rolling_duty_limit: f32,
    pub motor_positions_deg: [u16; 6],
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DeviceConfigSection {
    pub imu: ImuConfig,
    pub pins: PinConfig,
    pub calibration_static_s: u8,
    pub mahony_kp: f32,
    pub mahony_ki: f32,
    pub gait: GaitConfig,
    pub zupt: ZuptConfig,
    pub error: ErrorConfig,
    pub haptics: HapticConfig,
    pub ap_ip: [u8; 4],
    pub ws_port: u16,
    pub frames_per_batch: u8,
    pub storage_block_bytes: u16,
    pub storage_flush_ms: u16,
    pub storage_free_floor: f32,
    pub reference_min_cycles: u16,
    pub reference_check_cycles: u16,
}

fn mount_map(r: &mut Reader<'_>) -> Result<MountMap> {
    let mut map = [[0i8; 3]; 3];
    for row in map.iter_mut() {
        for value in row.iter_mut() {
            *value = r.i8()?;
        }
    }
    Ok(map)
}

fn f32_array<const N: usize>(r: &mut Reader<'_>) -> Result<[f32; N]> {
    let mut out = [0f32; N];
    for slot in out.iter_mut() {
        *slot = r.f32()?;
    }
    Ok(out)
}

pub fn parse_section(section: &[u8]) -> Result<DeviceConfigSection> {
    let mut r = Reader::new(section);
    let imu = ImuConfig {
        foot_address: r.u8()?,
        shank_address: r.u8()?,
        i2c_hz: r.u32()?,
        sample_hz: r.u16()?,
        accel_range_g: r.u8()?,
        gyro_range_dps: r.u16()?,
        dlpf_hz: r.u8()?,
        dlpf_cfg: r.u8()?,
        sample_rate_divider: r.u8()?,
        accel_lsb_per_g: r.f32()?,
        gyro_lsb_per_dps: r.f32()?,
        foot_mount: mount_map(&mut r)?,
        shank_mount: mount_map(&mut r)?,
    };
    let pins = PinConfig {
        i2c_sda: r.u8()?,
        i2c_scl: r.u8()?,
        foot_imu_int: r.u8()?,
        shank_imu_int: r.u8()?,
        motors: r.array::<6>()?,
    };
    let calibration_static_s = r.u8()?;
    let mahony_kp = r.f32()?;
    let mahony_ki = r.f32()?;
    let gait = GaitConfig {
        gait_lowpass_hz: r.f32()?,
        event_lowpass_hz: r.f32()?,
        min_cycle_s: r.f32()?,
        max_cycle_s: r.f32()?,
        contact_guard_ms: r.u16()?,
        toeoff_guard_ms: r.u16()?,
        ic_window_ms: r.u16()?,
    };
    let zupt = ZuptConfig {
        accel_tolerance_g: r.f32()?,
        gyro_threshold_dps: r.f32()?,
        min_duration_ms: r.u16()?,
        entry_hysteresis_ms: r.u16()?,
    };
    let error = ErrorConfig {
        class_activation: r.f32()?,
        haptic_on: r.f32()?,
        haptic_off: r.f32()?,
        confidence_haptic: r.f32()?,
        robust_z_clip: r.f32()?,
        weights: f32_array::<7>(&mut r)?,
    };
    let haptics = HapticConfig {
        fitted: r.u8()? != 0,
        pwm_hz: r.u16()?,
        pwm_bits: r.u8()?,
        min_duty: r.u8()?,
        max_duty: r.u8()?,
        intensity_exponent: r.f32()?,
        max_on_s: r.u8()?,
        rolling_window_s: r.u8()?,
        rolling_duty_limit: r.f32()?,
        motor_positions_deg: {
            let mut degrees = [0u16; 6];
            for slot in degrees.iter_mut() {
                *slot = r.u16()?;
            }
            degrees
        },
    };
    let config = DeviceConfigSection {
        imu,
        pins,
        calibration_static_s,
        mahony_kp,
        mahony_ki,
        gait,
        zupt,
        error,
        haptics,
        ap_ip: r.array::<4>()?,
        ws_port: r.u16()?,
        frames_per_batch: r.u8()?,
        storage_block_bytes: r.u16()?,
        storage_flush_ms: r.u16()?,
        storage_free_floor: r.f32()?,
        reference_min_cycles: r.u16()?,
        reference_check_cycles: r.u16()?,
    };
    // A section longer than this layout means the device speaks a newer format
    // than the declared one; treat that as malformed rather than guess.
    if r.remaining() != 0 {
        return Err(ProtocolError::BadPayload("configuration section"));
    }
    Ok(config)
}

impl DeviceConfigSection {
    /// Applies a sensor's mount map to chip-frame counts, giving anatomical
    /// physical units: X forward, Y medial (left), Z up (doc 04 §1).
    pub fn to_anatomical(map: &MountMap, counts: &[i16; 6], accel_lsb: f32, gyro_lsb: f32)
        -> ([f32; 3], [f32; 3])
    {
        let mut accel = [0f32; 3];
        let mut gyro = [0f32; 3];
        for axis in 0..3 {
            let mut a = 0f32;
            let mut g = 0f32;
            for source in 0..3 {
                let sign = map[axis][source] as f32;
                a += sign * counts[source] as f32;
                g += sign * counts[3 + source] as f32;
            }
            accel[axis] = a / accel_lsb;
            gyro[axis] = g / gyro_lsb;
        }
        (accel, gyro)
    }

    pub fn foot_anatomical(&self, counts: &[i16; 6]) -> ([f32; 3], [f32; 3]) {
        Self::to_anatomical(
            &self.imu.foot_mount,
            counts,
            self.imu.accel_lsb_per_g,
            self.imu.gyro_lsb_per_dps,
        )
    }

    pub fn shank_anatomical(&self, counts: &[i16; 6]) -> ([f32; 3], [f32; 3]) {
        Self::to_anatomical(
            &self.imu.shank_mount,
            counts,
            self.imu.accel_lsb_per_g,
            self.imu.gyro_lsb_per_dps,
        )
    }
}
