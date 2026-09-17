//! MATLAB Level-5 `.mat` writer (doc 10 §7, DEC-003).
//!
//! Written by hand rather than with a crate: the format is a handful of tagged
//! elements, and the alternative is a dependency that would have to be trusted
//! with the only copy of a session's data.
//!
//! The rules that matter and are easy to get wrong:
//!
//! * every data element is padded to an 8-byte boundary, and the padding is not
//!   counted in the element's own length;
//! * arrays are column-major;
//! * dimensions are (rows, columns), and a "row vector" of N values is 1×N;
//! * `miMATRIX` elements carry their sub-elements' padding inside their length;
//! * char arrays are UTF-16 (`miUTF16` inside an `mxCHAR_CLASS`);
//! * the header is 116 bytes of text, 8 reserved, then version `0x0100` and the
//!   endian marker `MI` — which is the two bytes `0x49 0x4D` on a little-endian
//!   writer, because MATLAB reads them as a big-endian `u16`.
//!
//! Raw counts stay integers all the way through (doc 10 §7: "the raw integer
//! values and timestamps must not be lost"), so `raw` is int32 and the
//! timestamps are int64, never doubles.

use crate::store::{Session, StatusChange, StoredCycle, StoredEvent, StoredReference};

const MI_INT8: u32 = 1;
const MI_INT32: u32 = 5;
const MI_UINT32: u32 = 6;
const MI_INT64: u32 = 12;
const MI_DOUBLE: u32 = 9;
const MI_MATRIX: u32 = 14;
// miUTF16 is 17. 18 is miUTF32, and a reader told the wrong one walks off the
// end of the buffer rather than failing cleanly — which is how this was found.
const MI_UTF16: u32 = 17;

const MX_CELL: u8 = 1;
const MX_STRUCT: u8 = 2;
const MX_CHAR: u8 = 4;
const MX_DOUBLE: u8 = 6;
const MX_INT32: u8 = 12;
const MX_INT64: u8 = 15;

/// One element: 4-byte type, 4-byte byte-length, the bytes, then padding to 8.
fn element(out: &mut Vec<u8>, kind: u32, bytes: &[u8]) {
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    pad(out);
}

fn pad(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(8) {
        out.push(0);
    }
}

/// Array flags: class byte, then the complex/global/logical bits (all clear).
fn flags(class: u8) -> Vec<u8> {
    let mut bytes = vec![0u8; 8];
    bytes[0] = class;
    bytes
}

fn dimensions(rows: usize, columns: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8);
    bytes.extend_from_slice(&(rows as i32).to_le_bytes());
    bytes.extend_from_slice(&(columns as i32).to_le_bytes());
    bytes
}

/// The name of a variable or field; MATLAB reads it as int8.
fn name(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// Wraps a matrix body as a top-level `miMATRIX` element.
fn matrix(body: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(&MI_MATRIX.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// A 1×N char array. MATLAB wants UTF-16, so a name with an accent survives.
fn char_array(variable: &str, text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut bytes = Vec::with_capacity(units.len() * 2);
    for unit in &units {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let mut body = Vec::new();
    element(&mut body, MI_UINT32, &flags(MX_CHAR));
    element(&mut body, MI_INT32, &dimensions(1, units.len()));
    element(&mut body, MI_INT8, &name(variable));
    element(&mut body, MI_UTF16, &bytes);
    matrix(body)
}

/// A rows×columns numeric array, written column-major.
fn numeric<T: Copy>(
    variable: &str,
    rows: usize,
    columns: usize,
    class: u8,
    kind: u32,
    values: &[T],
    encode: impl Fn(T) -> Vec<u8>,
) -> Vec<u8> {
    debug_assert_eq!(values.len(), rows * columns, "{variable}: {rows}x{columns}");
    let mut bytes = Vec::new();
    for value in values {
        bytes.extend_from_slice(&encode(*value));
    }
    let mut body = Vec::new();
    element(&mut body, MI_UINT32, &flags(class));
    element(&mut body, MI_INT32, &dimensions(rows, columns));
    element(&mut body, MI_INT8, &name(variable));
    element(&mut body, kind, &bytes);
    matrix(body)
}

/// Column-major from row-major: MATLAB reads down the columns.
fn transpose<T: Copy + Default>(rows: usize, columns: usize, row_major: &[T]) -> Vec<T> {
    let mut out = vec![T::default(); rows * columns];
    for r in 0..rows {
        for c in 0..columns {
            out[c * rows + r] = row_major[r * columns + c];
        }
    }
    out
}

fn doubles(variable: &str, rows: usize, columns: usize, row_major: &[f64]) -> Vec<u8> {
    let values = transpose(rows, columns, row_major);
    numeric(variable, rows, columns, MX_DOUBLE, MI_DOUBLE, &values, |v: f64| {
        v.to_le_bytes().to_vec()
    })
}

fn int64s(variable: &str, rows: usize, columns: usize, row_major: &[i64]) -> Vec<u8> {
    let values = transpose(rows, columns, row_major);
    numeric(variable, rows, columns, MX_INT64, MI_INT64, &values, |v: i64| {
        v.to_le_bytes().to_vec()
    })
}

fn int32s(variable: &str, rows: usize, columns: usize, row_major: &[i32]) -> Vec<u8> {
    let values = transpose(rows, columns, row_major);
    numeric(variable, rows, columns, MX_INT32, MI_INT32, &values, |v: i32| {
        v.to_le_bytes().to_vec()
    })
}

/// A 1×N cell array of char arrays, for a column of names.
fn cell_of_strings(variable: &str, items: &[String]) -> Vec<u8> {
    let mut cells = Vec::new();
    for item in items {
        cells.extend_from_slice(&char_array("", item));
    }
    let mut body = Vec::new();
    element(&mut body, MI_UINT32, &flags(MX_CELL));
    element(&mut body, MI_INT32, &dimensions(1, items.len()));
    element(&mut body, MI_INT8, &name(variable));
    body.extend_from_slice(&cells);
    matrix(body)
}

/// A 1×1 struct. `fields` are (name, an already-encoded miMATRIX with an empty
/// variable name), which is how MATLAB stores a struct's values.
fn structure(variable: &str, fields: &[(&str, Vec<u8>)]) -> Vec<u8> {
    // Field names are fixed-width; the width includes the terminating zero.
    let width = fields.iter().map(|(n, _)| n.len() + 1).max().unwrap_or(1).max(2) as i32;
    let mut names = Vec::new();
    for (field, _) in fields {
        let mut padded = vec![0u8; width as usize];
        padded[..field.len()].copy_from_slice(field.as_bytes());
        names.extend_from_slice(&padded);
    }
    let mut body = Vec::new();
    element(&mut body, MI_UINT32, &flags(MX_STRUCT));
    element(&mut body, MI_INT32, &dimensions(1, 1));
    element(&mut body, MI_INT8, &name(variable));
    element(&mut body, MI_INT32, &width.to_le_bytes());
    element(&mut body, MI_INT8, &names);
    for (_, value) in fields {
        body.extend_from_slice(value);
    }
    matrix(body)
}

fn header() -> Vec<u8> {
    let text = format!(
        "MATLAB 5.0 MAT-file, produced by the EAD V1 dashboard {}, doc 10 §7",
        env!("CARGO_PKG_VERSION")
    );
    let mut out = vec![b' '; 116];
    let bytes = text.as_bytes();
    out[..bytes.len().min(116)].copy_from_slice(&bytes[..bytes.len().min(116)]);
    out.extend_from_slice(&[0u8; 8]); // subsystem offset: unused
    out.extend_from_slice(&0x0100u16.to_le_bytes());
    // 'M','I' read as a big-endian u16: a reader that sees 'IM' knows to swap.
    out.extend_from_slice(b"IM");
    out
}

/// Builds `session.mat`: `raw`, `gait`, `events`, `haptics`, `metadata`,
/// `reference` (doc 10 §7).
pub fn session_mat(
    session: &Session,
    cycles: &[StoredCycle],
    events: &[StoredEvent],
    status: &[StatusChange],
    reference: Option<&StoredReference>,
    frames: impl FnOnce(&mut dyn FnMut(&crate::protocol::RawFrame)) -> super::Result<usize>,
) -> super::Result<Vec<u8>> {
    let mut out = header();

    // ---- raw: counts and timestamps as integers, never doubles -------------
    // Columns: timestamp_us, frame_index, sensor (0 foot, 1 shank),
    // ax..az, gx..gz, qw..qz, status_flags. Two rows per frame.
    let mut timestamps: Vec<i64> = Vec::new();
    let mut counts: Vec<i32> = Vec::new();
    let mut visit = |frame: &crate::protocol::RawFrame| {
        for (sensor, values, quaternion) in
            [(0i32, &frame.foot, &frame.q_foot), (1, &frame.shank, &frame.q_shank)]
        {
            timestamps.push(frame.timestamp_us as i64);
            counts.push(frame.frame_index as i32);
            counts.push(sensor);
            counts.extend(values.iter().map(|v| i32::from(*v)));
            counts.extend(quaternion.iter().map(|v| i32::from(*v)));
            counts.push(i32::from(frame.status));
        }
    };
    let rows = frames(&mut visit)? * 2;
    out.extend_from_slice(&structure(
        "raw",
        &[
            ("timestamp_us", int64s("", rows, 1, &timestamps)),
            ("values", int32s("", rows, 13, &counts)),
            (
                "columns",
                cell_of_strings(
                    "",
                    &[
                        "frame_index", "sensor", "ax", "ay", "az", "gx", "gy", "gz", "qw", "qx",
                        "qy", "qz", "status_flags",
                    ]
                    .map(String::from),
                ),
            ),
            (
                "units",
                char_array(
                    "",
                    "ADC counts in the chip frame (DEC-007); quaternions are Q15. \
Scale factors and mount maps are in metadata.json.",
                ),
            ),
            ("sensor_codes", char_array("", "0 = foot, 1 = shank")),
        ],
    ));

    // ---- gait ---------------------------------------------------------------
    let gait_columns = [
        "segment_id",
        "cycle_id",
        "start_us",
        "end_us",
        "cycle_time_s",
        "stance_s",
        "swing_s",
        "cadence_spm",
        "cycle_distance_m",
        "speed_mps",
        "unilateral_cycle_symmetry_proxy",
        "error_score",
        "confidence",
        "primary_error_class",
        "zupt_quality",
        "valid",
    ];
    let scored = session.reference_id.is_some();
    let mut gait = Vec::with_capacity(cycles.len() * gait_columns.len());
    for (index, c) in cycles.iter().enumerate() {
        let end_us = c.start_us + (c.cycle_time_s as f64 * 1e6) as i64;
        // NaN, not zero, for what was not measured: MATLAB's own missing value,
        // and `nanmedian` ignores it instead of averaging in a false agreement.
        let missing = f64::NAN;
        gait.extend_from_slice(&[
            c.segment_index as f64,
            (index + 1) as f64,
            c.start_us as f64,
            end_us as f64,
            c.cycle_time_s as f64,
            c.stance_time_s as f64,
            c.swing_time_s as f64,
            c.cadence_steps_per_min as f64,
            c.distance_m as f64,
            c.speed_mps as f64,
            c.symmetry_proxy.map(f64::from).unwrap_or(missing),
            if scored && c.confidence > 0.0 { c.error_score as f64 } else { missing },
            if scored && c.confidence > 0.0 { c.confidence as f64 } else { missing },
            if scored && c.confidence > 0.0 { c.primary_class as f64 } else { missing },
            c.zupt_quality as f64,
            f64::from(u8::from(c.valid)),
        ]);
    }
    out.extend_from_slice(&structure(
        "gait",
        &[
            ("values", doubles("", cycles.len(), gait_columns.len(), &gait)),
            ("columns", cell_of_strings("", &gait_columns.map(String::from))),
            (
                "error_classes",
                cell_of_strings("", &crate::protocol::ERROR_CLASSES.map(String::from)),
            ),
            (
                "note",
                char_array(
                    "",
                    "NaN means not measured, never agreement: the symmetry proxy needs a \
previous valid cycle and a reference's spreads, and the error columns need a reference. \
primary_error_class indexes error_classes from zero.",
                ),
            ),
        ],
    ));

    // ---- events -------------------------------------------------------------
    let kinds: Vec<String> = events.iter().map(|e| e.kind.clone()).collect();
    let mut event_values = Vec::with_capacity(events.len() * 2);
    for e in events {
        event_values.push(e.timestamp_us);
        event_values.push(e.frame_index);
    }
    out.extend_from_slice(&structure(
        "events",
        &[
            ("values", int64s("", events.len(), 2, &event_values)),
            ("columns", cell_of_strings("", &["timestamp_us", "frame_index"].map(String::from))),
            ("event_type", cell_of_strings("", &kinds)),
            (
                "note",
                char_array(
                    "",
                    "The device's own events. events.csv additionally carries the cycle \
bounds, error transitions and faults that doc 10 §4 names, derived from gait and status.",
                ),
            ),
        ],
    ));

    // ---- haptics: the shape, and why it is empty ----------------------------
    out.extend_from_slice(&structure(
        "haptics",
        &[
            ("values", doubles("", 0, 0, &[])),
            (
                "columns",
                cell_of_strings(
                    "",
                    &["timestamp_us", "motor_ids", "error_class", "pwm", "duration_ms"]
                        .map(String::from),
                ),
            ),
            (
                "note",
                char_array(
                    "",
                    "Empty: no ERM drivers are fitted (DEC-006). The motor GPIOs are held \
low and no haptic command has ever been issued. This is not a recording gap.",
                ),
            ),
        ],
    ));

    // ---- metadata -----------------------------------------------------------
    let faults: Vec<i64> = status.iter().flat_map(|s| [s.frame_index, i64::from(s.faults)]).collect();
    out.extend_from_slice(&structure(
        "metadata",
        &[
            ("patient_id", char_array("", &session.patient_id)),
            ("patient_name", char_array("", &session.patient_name)),
            ("session_id", char_array("", &session.session_id)),
            ("session_kind", char_array("", &session.kind)),
            ("started_at", char_array("", &session.started_at)),
            ("stopped_at", char_array("", session.stopped_at.as_deref().unwrap_or(""))),
            ("firmware_version", char_array("", session.firmware.as_deref().unwrap_or(""))),
            ("hardware_identifier", char_array("", session.device_mac.as_deref().unwrap_or(""))),
            ("config_sha256", char_array("", session.config_sha256.as_deref().unwrap_or(""))),
            (
                "protocol_version",
                doubles("", 1, 1, &[f64::from(crate::protocol::PROTOCOL_VERSION)]),
            ),
            ("payload_schema", doubles("", 1, 1, &[f64::from(crate::protocol::SCHEMA_VERSION)])),
            ("frames_stored", doubles("", 1, 1, &[session.frames_stored as f64])),
            ("frames_missing", doubles("", 1, 1, &[session.frames_missing as f64])),
            (
                "max_valid_cycles_per_segment",
                doubles("", 1, 1, &[session.max_cycles_per_segment.map(|v| v as f64).unwrap_or(f64::NAN)]),
            ),
            (
                "max_errors_per_segment",
                doubles("", 1, 1, &[session.max_errors_per_segment.map(|v| v as f64).unwrap_or(f64::NAN)]),
            ),
            ("status_changes", int64s("", status.len(), 2, &faults)),
            (
                "status_changes_columns",
                cell_of_strings("", &["frame_index", "faults"].map(String::from)),
            ),
            (
                "coordinate_convention",
                char_array("", "anatomical, right-handed: +X forward, +Y medial, +Z up"),
            ),
            (
                "note",
                char_array(
                    "",
                    "Research and engineering data. Not a validated clinical record. \
The full device configuration is in metadata.json beside this file.",
                ),
            ),
        ],
    ));

    // ---- reference ----------------------------------------------------------
    let fields: Vec<(&str, Vec<u8>)> = match reference {
        Some(r) => {
            let mut values = Vec::with_capacity(14);
            for f in &r.profile.features {
                values.push(f64::from(f.median));
                values.push(f64::from(f.spread));
            }
            vec![
                ("reference_profile_id", char_array("", &r.reference_id)),
                ("version", doubles("", 1, 1, &[r.version as f64])),
                ("cycles", doubles("", 1, 1, &[r.cycles as f64])),
                ("locked", doubles("", 1, 1, &[f64::from(u8::from(r.locked))])),
                ("values", doubles("", 7, 2, &values)),
                ("columns", cell_of_strings("", &["median", "spread"].map(String::from))),
                (
                    "features",
                    cell_of_strings("", &crate::protocol::FEATURE_NAMES.map(String::from)),
                ),
            ]
        }
        None => vec![(
            "note",
            char_array("", "none: this session was not scored against a reference profile"),
        )],
    };
    out.extend_from_slice(&structure("reference", &fields));

    Ok(out)
}
