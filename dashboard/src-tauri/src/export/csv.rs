//! The CSV half of the export package (doc 10 §2–§5).
//!
//! Columns are exactly as doc 10 names them, in its order. Where the device
//! produces something the doc's list does not name — a zero-velocity window has
//! an end as well as a start — the extra row is written and the addition is
//! declared in `metadata.json`, because an export that silently holds less than
//! the store is worse than one that holds a documented extra.

use std::fmt::Write as _;

use crate::store::{Session, StatusChange, StoredCycle, StoredEvent};

/// Quotes only when a value could otherwise break the row.
fn field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// `raw.csv`, doc 10 §2: one row per IMU sample, foot and shank sharing the
/// timestamp of the frame they came from.
///
/// The values are the stored ADC counts in the chip frame, not physical units
/// (DEC-007). `metadata.json` carries the scale factors and both mount maps, so
/// the conversion is one multiplication away and nothing in the chain has been
/// rounded on the way out.
pub fn raw_header() -> &'static str {
    "timestamp_us,frame_index,sensor,ax,ay,az,gx,gy,gz,qw,qx,qy,qz,status_flags\n"
}

pub fn raw_row(
    out: &mut String,
    timestamp_us: i64,
    frame_index: i64,
    sensor: &str,
    accel_gyro: &[i16; 6],
    quaternion: &[i16; 4],
    status: i64,
) {
    let _ = writeln!(
        out,
        "{timestamp_us},{frame_index},{sensor},{},{},{},{},{},{},{},{},{},{},{status}",
        accel_gyro[0],
        accel_gyro[1],
        accel_gyro[2],
        accel_gyro[3],
        accel_gyro[4],
        accel_gyro[5],
        quaternion[0],
        quaternion[1],
        quaternion[2],
        quaternion[3],
    );
}

/// `gait.csv`, doc 10 §3.
pub fn gait(session: &Session, cycles: &[StoredCycle], classes: &[&str]) -> String {
    let mut out = String::from(
        "session_id,segment_id,cycle_id,start_us,end_us,cycle_time_s,stance_s,swing_s,\
cadence_spm,cycle_distance_m,speed_mps,unilateral_symmetry_proxy,error_score,confidence,\
primary_error_class,zupt_quality\n",
    );
    let scored = session.reference_id.is_some();
    for (index, c) in cycles.iter().enumerate() {
        let end_us = c.start_us + (c.cycle_time_s as f64 * 1e6) as i64;
        let _ = write!(
            &mut out,
            "{},{},{},{},{},{:.4},{:.4},{:.4},{:.2},{:.4},{:.4},",
            field(&session.session_id),
            c.segment_index,
            index + 1,
            c.start_us,
            end_us,
            c.cycle_time_s,
            c.stance_time_s,
            c.swing_time_s,
            c.cadence_steps_per_min,
            c.distance_m,
            c.speed_mps,
        );
        // Empty, not zero, wherever the value was not measured: a reader
        // filtering on "no reference" must not find a column of zeroes that
        // looks like perfect agreement.
        match c.symmetry_proxy {
            Some(p) => {
                let _ = write!(&mut out, "{p:.4},");
            }
            None => out.push(','),
        }
        if scored && c.confidence > 0.0 {
            let _ = write!(
                &mut out,
                "{:.4},{:.4},{},",
                c.error_score,
                c.confidence,
                classes.get(c.primary_class as usize).copied().unwrap_or("unknown"),
            );
        } else {
            out.push_str(",,,");
        }
        let _ = writeln!(&mut out, "{:.4}", c.zupt_quality);
    }
    out
}

/// One row of `events.csv`.
struct Event {
    timestamp_us: i64,
    kind: &'static str,
    cycle_id: Option<usize>,
    segment_id: i64,
    quality: Option<f32>,
}

/// `events.csv`, doc 10 §4.
///
/// The device emits five event types; the doc's list has ten. The ones it can
/// be built from honestly are built: cycle bounds from the cycles, error
/// transitions from consecutive scored cycles, faults from the recorded status
/// changes. `SERVICE_TEST` never appears — there are no haptics to test
/// (DEC-006) — and its absence is stated in `metadata.json` rather than left for
/// a reader to wonder about.
pub fn events(
    session: &Session,
    cycles: &[StoredCycle],
    stored: &[StoredEvent],
    status: &[StatusChange],
) -> String {
    let mut rows: Vec<Event> = Vec::new();
    let end_of = |c: &StoredCycle| c.start_us + (c.cycle_time_s as f64 * 1e6) as i64;
    let locate = |us: i64| -> Option<(usize, &StoredCycle)> {
        cycles.iter().enumerate().find(|(_, c)| us >= c.start_us && us < end_of(c))
    };

    for e in stored {
        let kind = match e.kind.as_str() {
            "initial_contact" => "INITIAL_CONTACT",
            "toe_off" => "TOE_OFF",
            "foot_flat" => "FOOT_FLAT",
            "zupt_start" => "ZUPT_APPLIED",
            "zupt_end" => "ZUPT_END",
            _ => continue,
        };
        let found = locate(e.timestamp_us);
        rows.push(Event {
            timestamp_us: e.timestamp_us,
            kind,
            cycle_id: found.map(|(i, _)| i + 1),
            segment_id: found.map(|(_, c)| c.segment_index).unwrap_or(0),
            // The only per-event quality the device reports is the zero-velocity
            // quality of the cycle the event fell in; nothing defines one for
            // the others, so they are left empty rather than given a number.
            quality: found.map(|(_, c)| c.zupt_quality),
        });
    }

    let mut error_open = false;
    for (index, c) in cycles.iter().enumerate() {
        rows.push(Event {
            timestamp_us: c.start_us,
            kind: "CYCLE_START",
            cycle_id: Some(index + 1),
            segment_id: c.segment_index,
            quality: Some(c.zupt_quality),
        });
        rows.push(Event {
            timestamp_us: end_of(c),
            kind: "CYCLE_END",
            cycle_id: Some(index + 1),
            segment_id: c.segment_index,
            quality: Some(c.zupt_quality),
        });
        if session.reference_id.is_none() || c.confidence < super::CONFIDENCE_FOR_DISPLAY {
            continue;
        }
        // An error is active while consecutive displayable cycles carry a
        // class, and resolves on the first that does not.
        let active = c.primary_class != 0;
        if active && !error_open {
            rows.push(Event {
                timestamp_us: c.start_us,
                kind: "ERROR_ACTIVE",
                cycle_id: Some(index + 1),
                segment_id: c.segment_index,
                quality: Some(c.confidence),
            });
        } else if !active && error_open {
            rows.push(Event {
                timestamp_us: c.start_us,
                kind: "ERROR_RESOLVED",
                cycle_id: Some(index + 1),
                segment_id: c.segment_index,
                quality: Some(c.confidence),
            });
        }
        error_open = active;
    }

    // Faults are timestamped by the frame the device reported them at; the
    // cycle they fall in is the one whose frame range contains it.
    for change in status.iter().filter(|s| s.faults != 0) {
        let found = cycles
            .iter()
            .enumerate()
            .find(|(_, c)| change.frame_index >= c.start_frame && change.frame_index <= c.end_frame);
        rows.push(Event {
            timestamp_us: found.map(|(_, c)| c.start_us).unwrap_or(0),
            kind: "FAULT",
            cycle_id: found.map(|(i, _)| i + 1),
            segment_id: found.map(|(_, c)| c.segment_index).unwrap_or(0),
            quality: None,
        });
    }

    rows.sort_by_key(|r| (r.timestamp_us, r.kind));
    let mut out = String::from("session_id,segment_id,cycle_id,timestamp_us,event_type,quality\n");
    for r in rows {
        let _ = write!(
            &mut out,
            "{},{},{},{},{},",
            field(&session.session_id),
            r.segment_id,
            r.cycle_id.map(|c| c.to_string()).unwrap_or_default(),
            r.timestamp_us,
            r.kind,
        );
        match r.quality {
            Some(q) => {
                let _ = writeln!(&mut out, "{q:.4}");
            }
            None => out.push('\n'),
        }
    }
    out
}

/// `haptics.csv`, doc 10 §5: the header alone.
///
/// No ERM drivers are fitted (DEC-006), so the device has never commanded a
/// motor and there is nothing to write. The file exists with its columns so
/// that a tool reading the package finds the schema it expects and sees, rather
/// than infers, that no haptic event occurred.
pub fn haptics() -> &'static str {
    "session_id,segment_id,cycle_id,timestamp_us,motor_ids,error_class,pwm,duration_ms,\
error_score,confidence,reason\n"
}

