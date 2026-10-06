//! The CSV half of the export package (doc 10 §2–§5).
//!
//! Columns are exactly as doc 10 names them, in its order. Where the device
//! produces something the doc's list does not name — a zero-velocity window has
//! an end as well as a start — the extra row is written and the addition is
//! declared in `metadata.json`, because an export that silently holds less than
//! the store is worse than one that holds a documented extra.

use std::fmt::Write as _;

use crate::store::{Session, StatusChange, StoredCycle, StoredEvent, StoredHaptic};

/// Quotes only when a value could otherwise break the row.
pub fn field(value: &str) -> String {
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
///
/// `rv_*` is the sensor's game rotation vector as it reported it (Q14, chip
/// frame, device schema 6); the cells are empty for a frame recorded before
/// then, which carried none.
///
/// Every CSV carries `patient_name` (DEC-028, the user), so a file stands on its
/// own; `name` is the session's patient name already passed through `field`.
pub fn raw_header() -> &'static str {
    "patient_name,timestamp_us,frame_index,sensor,ax,ay,az,gx,gy,gz,qw,qx,qy,qz,status_flags,\
rv_real,rv_i,rv_j,rv_k\n"
}

/// The two `raw.csv` rows of one frame, foot then shank.
pub fn raw_rows(out: &mut String, name: &str, frame: &crate::protocol::RawFrame) {
    for (sensor, values, quaternion, rotation_vector) in [
        ("foot", &frame.foot, &frame.q_foot, frame.rv_foot),
        ("shank", &frame.shank, &frame.q_shank, frame.rv_shank),
    ] {
        let _ = write!(
            out,
            "{name},{},{},{sensor},{},{},{},{},{},{},{},{},{},{},{},",
            frame.timestamp_us,
            frame.frame_index,
            values[0],
            values[1],
            values[2],
            values[3],
            values[4],
            values[5],
            quaternion[0],
            quaternion[1],
            quaternion[2],
            quaternion[3],
            frame.status,
        );
        match rotation_vector {
            Some(q) => {
                let _ = writeln!(out, "{},{},{},{}", q[0], q[1], q[2], q[3]);
            }
            None => out.push_str(",,,\n"),
        }
    }
}

/// `accel_native.csv`: every accelerometer sample as measured, at the sensor's
/// own rate (RAW_ACCEL_BATCH, DEC-021). `raw.csv` carries the accelerometer
/// interpolated to each frame's time; this is what it was interpolated from.
///
/// Counts are the stored chip-frame values (DEC-007). The `_g` columns are the
/// same counts divided by the session's `accel_lsb_per_g`, still in the chip
/// frame: no mount map is applied. They are empty when the session stored no
/// configuration, rather than scaled by a guess.
pub fn accel_header() -> &'static str {
    "patient_name,timestamp_us,sensor,sequence,ax,ay,az,ax_g,ay_g,az_g\n"
}

pub fn accel_row(
    out: &mut String,
    name: &str,
    sample: &crate::protocol::AccelSample,
    lsb_per_g: Option<f32>,
) {
    let [ax, ay, az] = sample.accel;
    let sensor = if sample.sensor == 0 { "foot" } else { "shank" };
    let _ = write!(out, "{name},{},{sensor},{},{ax},{ay},{az},", sample.timestamp_us, sample.sequence);
    match lsb_per_g {
        Some(lsb) => {
            let g = |v: i16| f64::from(v) / f64::from(lsb);
            let _ = writeln!(out, "{:.6},{:.6},{:.6}", g(ax), g(ay), g(az));
        }
        None => out.push_str(",,\n"),
    }
}

/// `gait.csv`, doc 10 §3, plus `patient_name` (DEC-028) and `valid` at the end:
/// a cycle outside the temporal guards is exported but marked, as in `session.mat`,
/// so it is not averaged in as a measurement.
pub fn gait(session: &Session, cycles: &[StoredCycle], classes: &[&str]) -> String {
    let mut out = String::from(
        "session_id,patient_name,segment_id,cycle_id,start_us,end_us,cycle_time_s,stance_s,swing_s,\
cadence_spm,cycle_distance_m,speed_mps,unilateral_symmetry_proxy,error_score,confidence,\
primary_error_class,zupt_quality,valid\n",
    );
    let scored = session.reference_id.is_some();
    for (index, c) in cycles.iter().enumerate() {
        let end_us = c.start_us + (c.cycle_time_s as f64 * 1e6) as i64;
        let _ = write!(
            &mut out,
            "{},{},{},{},{},{},{:.4},{:.4},{:.4},{:.2},{:.4},{:.4},",
            field(&session.session_id),
            field(&session.patient_name),
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
        let _ = writeln!(&mut out, "{:.4},{}", c.zupt_quality, u8::from(c.valid));
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
/// changes. `SERVICE_TEST` never appears — motor service tests are refused while
/// a session runs (doc 07 §7) — and its absence is stated in `metadata.json`
/// rather than left for a reader to wonder about.
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

    // Faults are timestamped by the frame the device reported them at (its
    // device time; until 2026-10-07 the start of the cycle around it, or 0,
    // audit F-37); the cycle they fall in is the one whose frame range contains
    // it. Clears are not a doc 10 event type; `session.mat` holds every change.
    for change in status.iter().filter(|s| s.faults != 0) {
        let found = cycles
            .iter()
            .enumerate()
            .find(|(_, c)| change.frame_index >= c.start_frame && change.frame_index <= c.end_frame);
        rows.push(Event {
            timestamp_us: change
                .timestamp_us
                .or_else(|| found.map(|(_, c)| c.start_us))
                .unwrap_or(0),
            kind: "FAULT",
            cycle_id: found.map(|(i, _)| i + 1),
            segment_id: found.map(|(_, c)| c.segment_index).unwrap_or(0),
            quality: None,
        });
    }

    rows.sort_by_key(|r| (r.timestamp_us, r.kind));
    let mut out =
        String::from("session_id,patient_name,segment_id,cycle_id,timestamp_us,event_type,quality\n");
    for r in rows {
        let _ = write!(
            &mut out,
            "{},{},{},{},{},{},",
            field(&session.session_id),
            field(&session.patient_name),
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

/// `haptics.csv`, doc 10 §5, one row per feedback record (DEC-023). `event`
/// (on, update, off) is added after doc 10's columns: doc 06 §13 asks for every
/// ON, update and OFF to be recorded, and the columns alone cannot tell them
/// apart. `cycle_id` and `segment_id` follow `events.csv`: the cycle's position,
/// 1-based, and its segment; empty for an episode ended outside a cycle.
/// Motors and duties are `;`-separated in the same order; a duty of 0 is a cue
/// held back while the laptop was not heard (reason `link_lost`, DEC-027) or one
/// the motor guard refused.
pub fn haptics(session: &Session, cycles: &[StoredCycle], records: &[StoredHaptic]) -> String {
    let mut out = String::from(
        "session_id,patient_name,segment_id,cycle_id,timestamp_us,motor_ids,error_class,pwm,\
duration_ms,error_score,confidence,reason,event\n",
    );
    for h in records {
        let cycle = (h.cycle_start_frame != 0)
            .then(|| cycles.iter().position(|c| c.start_frame == h.cycle_start_frame))
            .flatten();
        let motors: Vec<(i64, i64)> = [(h.motor_a, h.duty_a), (h.motor_b, h.duty_b)]
            .into_iter()
            .filter(|&(motor, _)| motor != 0)
            .collect();
        let join = |pick: fn(&(i64, i64)) -> i64| {
            motors.iter().map(|m| pick(m).to_string()).collect::<Vec<_>>().join(";")
        };
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            field(&session.session_id),
            field(&session.patient_name),
            cycle.map(|i| cycles[i].segment_index.to_string()).unwrap_or_default(),
            cycle.map(|i| (i + 1).to_string()).unwrap_or_default(),
            h.device_time_us,
            join(|m| m.0),
            field(&h.error_class),
            join(|m| m.1),
            h.duration_ms,
            h.error_score,
            h.confidence,
            h.reason,
            h.event,
        );
    }
    out
}

