//! The export package (doc 10): `raw.csv`, `gait.csv`, `events.csv`,
//! `haptics.csv`, `metadata.json`, `session.mat` and the PDF report.
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

/// Doc 06 §7, mirrored from `ead::kConfidenceForDisplay`.
pub const CONFIDENCE_FOR_DISPLAY: f32 = 0.50;

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
    pub gait_rows: usize,
    pub event_rows: usize,
}

/// Writes the whole package for one session into `directory`.
pub fn export_session(store: &Store, session_id: &str, directory: &Path) -> Result<ExportSummary> {
    std::fs::create_dir_all(directory)
        .map_err(|source| ExportError::Io { path: directory.to_path_buf(), source })?;

    let session = store.session(session_id)?;
    let cycles = store.cycles(session_id)?;
    let events = store.events(session_id)?;
    let segments = store.segments(session_id)?;
    let status = store.status_changes(session_id)?;
    let reference = match session.reference_id.as_deref() {
        Some(id) => Some(store.reference(id)?),
        None => None,
    };

    // raw.csv is built by streaming, so an hour-long session never lands in
    // memory twice.
    let mut raw = String::from(csv::raw_header());
    let frames = store.for_each_frame(session_id, |frame| {
        let status = i64::from(frame.status);
        let timestamp = frame.timestamp_us as i64;
        let index = i64::from(frame.frame_index);
        csv::raw_row(&mut raw, timestamp, index, "foot", &frame.foot, &frame.q_foot, status);
        csv::raw_row(&mut raw, timestamp, index, "shank", &frame.shank, &frame.q_shank, status);
    })?;

    let gait = csv::gait(&session, &cycles, &crate::protocol::ERROR_CLASSES);
    let event_csv = csv::events(&session, &cycles, &events, &status);
    let metadata = metadata_json(store, &session, &reference, &segments, frames, &cycles)?;

    let mut files = Vec::new();
    for (name, contents) in [
        ("raw.csv", raw.as_str()),
        ("gait.csv", gait.as_str()),
        ("events.csv", event_csv.as_str()),
        ("haptics.csv", csv::haptics()),
        ("metadata.json", metadata.as_str()),
    ] {
        write(directory, name, contents)?;
        files.push(name.to_string());
    }

    let mat = mat::session_mat(&session, &cycles, &events, &status, reference.as_ref(), |visit| {
        store.for_each_frame(session_id, visit).map_err(ExportError::Store)
    })?;
    write(directory, "session.mat", mat)?;
    files.push("session.mat".into());

    let report = pdf::report(
        &session,
        &cycles,
        &segments,
        &status,
        reference.as_ref(),
        &crate::protocol::ERROR_CLASSES,
    )?;
    write(directory, "report.pdf", report)?;
    files.push("report.pdf".into());

    Ok(ExportSummary {
        directory: directory.display().to_string(),
        files,
        raw_rows: frames * 2,
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
    cycles: &[crate::store::StoredCycle],
) -> Result<String> {
    let config = store
        .session_config(&session.session_id)?
        .and_then(|bytes| crate::protocol::parse_section(&bytes).ok());
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
            "kind": session.kind,
            "started_at": session.started_at,
            "stopped_at": session.stopped_at,
            "frames_stored": session.frames_stored,
            "frames_missing": session.frames_missing,
            "first_frame_index": session.first_frame_index,
            "last_frame_index": session.last_frame_index,
            "cycles": cycles.len(),
            "valid_cycles": cycles.iter().filter(|c| c.valid).count(),
        },
        "device": {
            "firmware_version": session.firmware,
            "protocol_version": crate::protocol::PROTOCOL_VERSION,
            "payload_schema": crate::protocol::SCHEMA_VERSION,
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
            "fitted": false,
            "reason": "no ERM drivers are fitted (DEC-006); the motor GPIOs are held low \
and no haptic command has ever been issued",
            "safety_configuration": "not applicable: no driver, no PWM, no motor service",
        },
        "storage_recovery": {
            "on_device_flash": "not implemented in V1 (M7 deferred)",
            "state": "none",
            "gap_recovery": "dropped messages are re-requested from the device's RAM ring \
while the link is up; frames_missing above is what never arrived",
        },
        "export_notes": {
            "raw_csv": format!("{frames} frames, two rows each (foot, shank)"),
            "events_csv": "ZUPT_END is written in addition to doc 10 §4's list: the device \
reports zero-velocity windows, not instants, and dropping the end would lose the \
window length. SERVICE_TEST never appears, because there are no haptics to test. \
The quality column carries the zero-velocity quality of the cycle the event fell in, \
and is empty for an event that fell in no cycle.",
            "haptics_csv": "header only; see haptics above",
            "gait_csv": "empty cells are values that were not measured, never zeroes: \
symmetry proxy needs a previous valid cycle and a reference's spreads, and the error \
columns need a reference",
        },
    });
    serde_json::to_string_pretty(&value).map_err(|e| ExportError::Encode(e.to_string()))
}
