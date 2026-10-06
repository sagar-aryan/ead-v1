//! The export package (doc 10): `raw.csv`, `gait.csv`, `events.csv`,
//! `haptics.csv`, `metadata.json`, `session.mat` and the PDF report, plus
//! `accel_native.csv`, the accelerometer at its own rate (DEC-021).
//!
//! Everything here is derived from the store, never from a live device, so a
//! session recorded last month exports the same bytes today. Where the device
//! produced nothing — haptics, service tests — the file or field still exists
//! and says so, because a reader must be able to tell "did not happen" from
//! "was not recorded".

pub mod csv;
pub mod mat;
pub mod pdf;

use std::path::{Path, PathBuf};

use crate::store::{Store, StoredReference};

use crate::store::CONFIDENCE_FOR_DISPLAY;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("{0}")]
    Store(#[from] crate::store::StoreError),
    #[error("writing {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{0}")]
    Encode(String),
}

type Result<T> = std::result::Result<T, ExportError>;

/// Writes a CSV row by row: `rows` calls its argument once per row's text. The
/// raw and accelerometer files of an hour's session are hundreds of MB, and
/// were built whole in memory first (PROB-035).
fn write_rows(
    dir: &Path,
    name: &str,
    header: &str,
    rows: impl FnOnce(&mut dyn FnMut(&str)) -> Result<usize>,
) -> Result<usize> {
    use std::io::Write;
    let path = dir.join(name);
    let io = |source| ExportError::Io { path: path.clone(), source };
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).map_err(io)?);
    out.write_all(header.as_bytes()).map_err(io)?;
    let mut failed = None;
    let count = rows(&mut |text| {
        if failed.is_none() {
            failed = out.write_all(text.as_bytes()).err();
        }
    })?;
    if let Some(source) = failed {
        return Err(io(source));
    }
    out.flush().map_err(io)?;
    Ok(count)
}

fn write(dir: &Path, name: &str, contents: impl AsRef<[u8]>) -> Result<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, contents).map_err(|source| ExportError::Io { path: path.clone(), source })?;
    Ok(path)
}

/// What an export produced, for the UI to show and for the tests to check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportSummary {
    pub directory: String,
    pub files: Vec<String>,
    pub raw_rows: usize,
    /// Rows of `accel_native.csv`: one per accelerometer sample.
    pub accel_rows: usize,
    pub gait_rows: usize,
    pub event_rows: usize,
}

/// Writes the whole package for one session into `directory`.
pub fn export_session(store: &Store, session_id: &str, directory: &Path) -> Result<ExportSummary> {
    // A session still recording changes between the reads below, and its files
    // disagreed with one another (metadata 10,000 frames, CSV 10,300: PROB-035).
    // A stopped one does not change.
    if store.recording_session().as_deref() == Some(session_id) {
        return Err(ExportError::Store(crate::store::StoreError::Rejected(format!(
            "session {session_id} is still recording; stop it, then export"
        ))));
    }
    std::fs::create_dir_all(directory)
        .map_err(|source| ExportError::Io { path: directory.to_path_buf(), source })?;

    let session = store.session(session_id)?;
    let cycles = store.cycles(session_id)?;
    let events = store.events(session_id)?;
    let segments = store.segments(session_id)?;
    let haptics = store.haptics(session_id)?;
    let status = store.status_changes(session_id)?;
    let reference = match session.reference_id.as_deref() {
        Some(id) => Some(store.reference(id)?),
        None => None,
    };

    let mut files = Vec::new();
    let mut line = String::new();
    let name = csv::field(&session.patient_name);
    let frames = write_rows(directory, "raw.csv", csv::raw_header(), |emit| {
        Ok(store.for_each_frame(session_id, |frame| {
            line.clear();
            csv::raw_rows(&mut line, &name, frame);
            emit(&line);
        })?)
    })?;
    files.push("raw.csv".to_string());

    let lsb_per_g = store
        .session_config(session_id)?
        .and_then(|section| section.parse().ok())
        .map(|config| config.imu.accel_lsb_per_g);
    let accel_rows = write_rows(directory, "accel_native.csv", csv::accel_header(), |emit| {
        Ok(store.for_each_accel(session_id, |sample| {
            line.clear();
            csv::accel_row(&mut line, &name, sample, lsb_per_g);
            emit(&line);
        })?)
    })?;
    files.push("accel_native.csv".to_string());

    let gait = csv::gait(&session, &cycles, &crate::protocol::ERROR_CLASSES);
    let event_csv = csv::events(&session, &cycles, &events, &status);
    let metadata =
        metadata_json(store, &session, &reference, &segments, frames, accel_rows, &cycles)?;
    let haptic_csv = csv::haptics(&session, &cycles, &haptics);

    for (name, contents) in [
        ("gait.csv", gait.as_str()),
        ("events.csv", event_csv.as_str()),
        ("haptics.csv", haptic_csv.as_str()),
        ("metadata.json", metadata.as_str()),
    ] {
        write(directory, name, contents)?;
        files.push(name.to_string());
    }

    let mat = mat::session_mat(
        &session,
        &cycles,
        &events,
        &status,
        reference.as_ref(),
        &haptics,
        |visit| store.for_each_frame(session_id, visit).map_err(ExportError::Store),
        |visit| store.for_each_accel(session_id, visit).map_err(ExportError::Store),
    )?;
    write(directory, "session.mat", mat)?;
    files.push("session.mat".into());

    let report = pdf::report(
        &session,
        &cycles,
        &segments,
        &status,
        reference.as_ref(),
        &haptics,
        &crate::protocol::ERROR_CLASSES,
    )?;
    write(directory, "report.pdf", report)?;
    files.push("report.pdf".into());

    Ok(ExportSummary {
        directory: directory.display().to_string(),
        files,
        raw_rows: frames * 2,
        accel_rows,
        gait_rows: cycles.len(),
        event_rows: event_csv.lines().count().saturating_sub(1),
    })
}

/// `metadata.json`, doc 10 §6.
///
/// Every field the doc names is present. Where V1 has nothing to report the
/// field says so in words — `"not fitted"`, `"none"` — rather than being
/// omitted, so a reader can tell the difference between a system that had no
/// haptics and an export that forgot to mention them.
fn metadata_json(
    store: &Store,
    session: &crate::store::Session,
    reference: &Option<StoredReference>,
    segments: &[crate::store::StoredSegment],
    frames: usize,
    accel_samples: usize,
    cycles: &[crate::store::StoredCycle],
) -> Result<String> {
    let haptics = store.haptics(&session.session_id)?;
    let config = store
        .session_config(&session.session_id)?
        .and_then(|section| section.parse().ok());
    let calibration: Option<serde_json::Value> = store
        .session_calibration(&session.session_id)?
        .and_then(|text| serde_json::from_str(&text).ok());

    let value = serde_json::json!({
        "document": {
            "produced_by": concat!("EAD V1 dashboard ", env!("CARGO_PKG_VERSION")),
            "schema": "docs/protocol.md; layout per ead_agent_docs_v2/10_DATA_SCHEMA_AND_EXPORT.md",
            "note": "Research and engineering data. Not a validated clinical record.",
        },
        "patient": { "patient_id": session.patient_id, "patient_name": session.patient_name },
        "session": {
            "session_id": session.session_id,
            "label": session.label,
            "kind": session.kind,
            "started_at": session.started_at,
            "stopped_at": session.stopped_at,
            "frames_stored": session.frames_stored,
            "accel_samples_stored": accel_samples,
            "frames_missing": session.frames_missing,
            "first_frame_index": session.first_frame_index,
            "last_frame_index": session.last_frame_index,
            "cycles": cycles.len(),
            "valid_cycles": cycles.iter().filter(|c| c.valid).count(),
        },
        "device": {
            "firmware_version": session.firmware,
            "protocol_version": crate::protocol::PROTOCOL_VERSION,
            // The schema the device recorded at, not this app's (PROB-035).
            "payload_schema": session.payload_schema,
            "hardware_identifier": session.device_mac,
            "boot_id": session.boot_id,
            "config_sha256": session.config_sha256,
            // Addresses, the fixed GPIO map, ranges, DLPF, sample rate and both
            // mount maps, exactly as the device reported them (doc 10 §6).
            "configuration": config,
        },
        "coordinate_convention": {
            "frame": "anatomical, right-handed: +X forward, +Y medial, +Z up",
            "raw_csv": "chip-frame ADC counts as stored (DEC-007), not physical units",
            "conversion": "divide by accel_lsb_per_g / gyro_lsb_per_dps, then apply the \
sensor's mount map; both are under device.configuration",
            "quaternions": "Q15 signed 16-bit, w x y z; divide by 32767",
            "rotation_vectors": "raw.csv rv_real..rv_k: the sensor's game rotation vector as \
reported, Q14 signed 16-bit, real i j k, chip frame; divide by 16384. Empty for frames \
recorded before device schema 6",
            "accel_native_csv": "chip-frame ADC counts as measured; the _g columns divide by \
accel_lsb_per_g and apply no mount map",
        },
        "calibration": calibration.unwrap_or(serde_json::json!(
            "none recorded: the session started with no usable calibration record"
        )),
        "reference_profile": match reference {
            Some(r) => serde_json::json!({
                "reference_profile_id": r.reference_id,
                "version": r.version,
                "cycles": r.cycles,
                "locked": r.locked,
                "captured_at": r.created_at,
                "captured_in_session": r.session_id,
                "features": crate::protocol::FEATURE_NAMES
                    .iter()
                    .zip(r.profile.features.iter())
                    .map(|(name, f)| serde_json::json!({
                        "name": name, "median": f.median, "spread": f.spread
                    }))
                    .collect::<Vec<_>>(),
            }),
            None => serde_json::json!("none: this session was not scored against a reference"),
        },
        "segmentation": {
            "max_valid_cycles_per_segment": session.max_cycles_per_segment,
            "max_errors_per_segment": session.max_errors_per_segment,
            "counted_on": "the host (DEC-014)",
            "error_definition": "a scored cycle with a primary class other than NONE and \
confidence at or above 0.50, the level below which doc 06 §7 says the classification \
should not be shown",
            "segments": segments,
        },
        "haptics": {
            // The device's own configuration says whether feedback could run;
            // what ran is in haptics.csv.
            "fitted": config.as_ref().map(|c| c.haptics.fitted),
            "safety_configuration": config.as_ref().map(|c| &c.haptics),
            "records": haptics.len(),
            "cues_run": haptics.iter().filter(|h| h.event != "off" && h.duty_a > 0).count(),
            "episodes": haptics.iter().filter(|h| h.event == "on").count(),
            "delivery": "one cue per scored cycle when the cycle closes (the right foot \
lands), in an EVALUATION session with the dashboard's master switch on (DEC-023)",
        },
        "storage_recovery": {
            "on_device_flash": "not implemented in V1 (M7 deferred)",
            "state": "none",
            "gap_recovery": "dropped messages are re-requested from the device's RAM ring \
while the link is up; frames_missing above is what never arrived",
        },
        "export_notes": {
            "raw_csv": format!("{frames} frames, two rows each (foot, shank)"),
            "accel_native_csv": format!("{accel_samples} accelerometer samples at the \
sensors' native rate (RAW_ACCEL_BATCH, DEC-021); raw.csv's accelerometer columns are \
these interpolated to each frame's time. Empty (header only) for a session recorded \
before device schema 6. A gap in a sensor's sequence column (it wraps at 256) is a \
lost sample"),
            "events_csv": "ZUPT_END is written in addition to doc 10 §4's list: the device \
reports zero-velocity windows, not instants, and dropping the end would lose the \
window length. SERVICE_TEST never appears: motor service tests are refused while a session runs. \
The quality column carries the zero-velocity quality of the cycle the event fell in, \
and is empty for an event that fell in no cycle.",
            "haptics_csv": "one row per feedback record; `event` (on, update, off) is \
added after doc 10's columns because doc 06 §13 asks for every ON, update and OFF. \
Header only when no cue ran",
            "gait_csv": "empty cells are values that were not measured, never zeroes: \
symmetry proxy needs a previous valid cycle and a reference's spreads, and the error \
columns need a reference",
        },
    });
    serde_json::to_string_pretty(&value).map_err(|e| ExportError::Encode(e.to_string()))
}
