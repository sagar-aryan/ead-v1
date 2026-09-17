//! Decimated reads over stored frames, for the raw-data view.
//!
//! An hour of recording is 360,000 frames and a chart has a few thousand pixels,
//! so frames are aggregated into buckets and each bucket reports its minimum and
//! maximum. That keeps transients visible: a single-sample impact survives
//! decimation, where sampling every Nth frame would drop it.
//!
//! Counts are converted to anatomical physical units here, using the
//! configuration stored with the session, so the view needs no knowledge of
//! mount maps and a session recorded under different settings still reads
//! correctly.

use rusqlite::Connection;

use super::{Result, StoreError};
use crate::protocol::config::{DeviceConfigSection, MountMap};

/// Signal groups the raw view can request: one sensor, one quantity, three axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalGroup {
    FootAccel,
    FootGyro,
    ShankAccel,
    ShankGyro,
}

impl SignalGroup {
    /// Database columns for the sensor's three chip axes, in X, Y, Z order.
    fn columns(self) -> [&'static str; 3] {
        match self {
            SignalGroup::FootAccel => ["fax", "fay", "faz"],
            SignalGroup::FootGyro => ["fgx", "fgy", "fgz"],
            SignalGroup::ShankAccel => ["sax", "say", "saz"],
            SignalGroup::ShankGyro => ["sgx", "sgy", "sgz"],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SignalGroup::FootAccel => "Foot acceleration",
            SignalGroup::FootGyro => "Foot angular rate",
            SignalGroup::ShankAccel => "Shank acceleration",
            SignalGroup::ShankGyro => "Shank angular rate",
        }
    }

    pub fn sensor(self) -> &'static str {
        match self {
            SignalGroup::FootAccel | SignalGroup::FootGyro => "foot",
            SignalGroup::ShankAccel | SignalGroup::ShankGyro => "shank",
        }
    }

    fn is_accel(self) -> bool {
        matches!(self, SignalGroup::FootAccel | SignalGroup::ShankAccel)
    }

    fn mount<'a>(self, config: &'a DeviceConfigSection) -> &'a MountMap {
        match self.sensor() {
            "foot" => &config.imu.foot_mount,
            _ => &config.imu.shank_mount,
        }
    }

    fn scale(self, config: &DeviceConfigSection) -> f32 {
        if self.is_accel() {
            config.imu.accel_lsb_per_g
        } else {
            config.imu.gyro_lsb_per_dps
        }
    }

    fn unit(self, anatomical: bool) -> &'static str {
        if !anatomical {
            "counts"
        } else if self.is_accel() {
            "g"
        } else {
            "°/s"
        }
    }
}

/// Below this many frames the raw rows are returned unaggregated.
const EXACT_BELOW: i64 = 4000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AxisWindow {
    /// Anatomical axis when converted, chip axis when not: X, Y or Z.
    pub axis: &'static str,
    pub min: Vec<f32>,
    pub max: Vec<f32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RawWindow {
    pub session_id: String,
    pub group: SignalGroup,
    pub label: &'static str,
    pub sensor: &'static str,
    pub unit: &'static str,
    /// False when the session has no stored configuration: values are raw counts
    /// in the sensor's own axes, not anatomical physical units.
    pub anatomical: bool,
    pub first_frame: i64,
    pub last_frame: i64,
    /// Frames per point: 1 means every frame is shown exactly.
    pub bucket: i64,
    /// Device time of each point, seconds since the session's first frame.
    pub time_s: Vec<f64>,
    pub frame_index: Vec<i64>,
    pub axes: Vec<AxisWindow>,
    /// Status flags OR-ed within each bucket, so a flagged frame is never hidden.
    pub status: Vec<i64>,
    pub points: usize,
    pub query_ms: u64,
}

/// One bucket's chip-frame extremes, before the mount map is applied.
struct Extremes {
    min: [f32; 3],
    max: [f32; 3],
}

/// Applies a mount map to a bucket's extremes.
///
/// Each anatomical axis is ±1 times one chip axis. A positive sign carries the
/// chip minimum to the anatomical minimum; a negative sign swaps them, which is
/// the case that makes this worth doing in one tested place.
fn map_extremes(map: &MountMap, scale: f32, bucket: &Extremes) -> ([f32; 3], [f32; 3]) {
    let mut min = [0f32; 3];
    let mut max = [0f32; 3];
    for axis in 0..3 {
        let mut lo = 0f32;
        let mut hi = 0f32;
        for source in 0..3 {
            let sign = map[axis][source] as f32;
            if sign == 0.0 {
                continue;
            }
            let a = bucket.min[source] * sign;
            let b = bucket.max[source] * sign;
            lo += a.min(b);
            hi += a.max(b);
        }
        min[axis] = lo / scale;
        max[axis] = hi / scale;
    }
    (min, max)
}

pub fn read_window(
    connection: &Connection,
    session_id: &str,
    group: SignalGroup,
    first_frame: i64,
    last_frame: i64,
    max_points: usize,
    config: Option<&DeviceConfigSection>,
) -> Result<RawWindow> {
    if last_frame < first_frame {
        return Err(StoreError::Rejected("empty frame range".into()));
    }
    let started = std::time::Instant::now();

    let span = last_frame - first_frame + 1;
    let max_points = max_points.clamp(1, 20_000) as i64;
    let bucket = if span <= EXACT_BELOW { 1 } else { (span / max_points).max(1) };

    // Time is relative to the session's first frame, which is what a reader
    // wants on the axis; the absolute device time stays in the store.
    let origin_us: i64 = connection
        .query_row(
            "SELECT MIN(timestamp_us) FROM raw_frames WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Column names come from the enum, never from the caller.
    let columns = group.columns();
    let aggregates = columns
        .iter()
        .map(|c| format!("MIN({c}), MAX({c})"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT MIN(timestamp_us), MIN(frame_index), {aggregates}, MAX(status)
         FROM raw_frames
         WHERE session_id = ?1 AND frame_index BETWEEN ?2 AND ?3
         GROUP BY frame_index / ?4 ORDER BY frame_index / ?4"
    );

    let mut statement = connection.prepare(&sql)?;
    let mut rows =
        statement.query(rusqlite::params![session_id, first_frame, last_frame, bucket])?;

    let identity: MountMap = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
    let (map, scale) = match config {
        Some(config) => (group.mount(config), group.scale(config)),
        None => (&identity, 1.0),
    };

    let axis_names = ["X", "Y", "Z"];
    let mut window = RawWindow {
        session_id: session_id.to_string(),
        group,
        label: group.label(),
        sensor: group.sensor(),
        unit: group.unit(config.is_some()),
        anatomical: config.is_some(),
        first_frame,
        last_frame,
        bucket,
        time_s: Vec::new(),
        frame_index: Vec::new(),
        axes: axis_names
            .iter()
            .map(|axis| AxisWindow { axis, min: Vec::new(), max: Vec::new() })
            .collect(),
        status: Vec::new(),
        points: 0,
        query_ms: 0,
    };

    while let Some(row) = rows.next()? {
        let timestamp_us: i64 = row.get(0)?;
        window.time_s.push((timestamp_us - origin_us) as f64 / 1e6);
        window.frame_index.push(row.get(1)?);
        let mut bucket_values = Extremes { min: [0.0; 3], max: [0.0; 3] };
        for source in 0..3 {
            bucket_values.min[source] = row.get::<_, i64>(2 + source * 2)? as f32;
            bucket_values.max[source] = row.get::<_, i64>(3 + source * 2)? as f32;
        }
        let (min, max) = map_extremes(map, scale, &bucket_values);
        for axis in 0..3 {
            window.axes[axis].min.push(min[axis]);
            window.axes[axis].max.push(max[axis]);
        }
        window.status.push(row.get(8)?);
    }
    window.points = window.time_s.len();
    window.query_ms = started.elapsed().as_millis() as u64;
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_negative_mount_sign_swaps_the_extremes() {
        // Shank map: X = -chipZ, Y = +chipX, Z = -chipY (DEC-009).
        let map: MountMap = [[0, 0, -1], [1, 0, 0], [0, -1, 0]];
        let bucket = Extremes { min: [-100.0, -200.0, -300.0], max: [100.0, 200.0, 300.0] };
        let (min, max) = map_extremes(&map, 10.0, &bucket);

        // Anatomical X is -chipZ: the chip maximum becomes the anatomical minimum.
        assert_eq!((min[0], max[0]), (-30.0, 30.0));
        // Anatomical Y is +chipX: order preserved.
        assert_eq!((min[1], max[1]), (-10.0, 10.0));
        assert_eq!((min[2], max[2]), (-20.0, 20.0));

        // An asymmetric bucket shows the swap unambiguously.
        let bucket = Extremes { min: [0.0, 0.0, 100.0], max: [0.0, 0.0, 800.0] };
        let (min, max) = map_extremes(&map, 1.0, &bucket);
        assert_eq!((min[0], max[0]), (-800.0, -100.0), "negative sign must swap min and max");
    }

    #[test]
    fn identity_mount_only_scales() {
        let map: MountMap = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
        let bucket = Extremes { min: [-8192.0, 0.0, 4096.0], max: [8192.0, 100.0, 8192.0] };
        let (min, max) = map_extremes(&map, 8192.0, &bucket);
        assert_eq!(min, [-1.0, 0.0, 0.5]);
        assert_eq!(max, [1.0, 100.0 / 8192.0, 1.0]);
    }
}
