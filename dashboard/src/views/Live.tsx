/**
 * LIVE: what the two sensors are reading right now.
 *
 * Gait metrics (cadence, stance/swing, error score) are not shown because the
 * device does not compute them yet — milestones M4 and M5. Showing an empty
 * placeholder for them would suggest the measurement exists.
 */
import { Strip } from "../components/Strip";
import type { DeviceApi } from "../useDevice";
import { Orientation } from "./Orientation";

const FOOT = "#2a78d6";
const SHANK = "#eb6834";
const AXIS_Y = "#4a3aa7";
const AXIS_Z = "#1baf7a";

// Stable identities: Strip rebuilds its chart when these change, and this view
// re-renders at 5 Hz.
const MAGNITUDE_SERIES = [
  { label: "foot", color: FOOT },
  { label: "shank", color: SHANK },
];
const FOOT_AXES = [
  { label: "x", color: FOOT },
  { label: "y", color: AXIS_Y },
  { label: "z", color: AXIS_Z },
];
const SHANK_AXES = [
  { label: "x", color: SHANK },
  { label: "y", color: AXIS_Y },
  { label: "z", color: AXIS_Z },
];
const MAGNITUDE_RANGE: [number, number] = [0, 2];

function Axis({ label, value, unit }: { label: string; value: number | null; unit: string }) {
  return (
    <div className="readout">
      <span className="label">{label}</span>
      <span className="value num">
        {value === null ? (
          <span className="absent">—</span>
        ) : (
          <>
            {value.toFixed(Math.abs(value) < 100 ? 2 : 0)}
            <span className="unit">{unit}</span>
          </>
        )}
      </span>
    </div>
  );
}

export function Live({ device }: { device: DeviceApi }) {
  const { snapshot, tick, rings, config, vocabulary } = device;
  const connected = snapshot?.link_state === "connected";
  const flags =
    tick && vocabulary
      ? vocabulary.raw_status_names.filter((_, bit) => (tick.status_flags & (1 << bit)) !== 0)
      : [];

  if (!connected) {
    return (
      <>
        <h1>Live</h1>
        <p className="page-hint">
          Connect the device from the Device view to see the sensors.
        </p>
      </>
    );
  }

  const units = tick?.anatomical ?? false;
  const accelUnit = units ? "g" : "counts";
  const gyroUnit = units ? "°/s" : "counts";

  return (
    <>
      <h1>Live</h1>
      <p className="page-hint">
        Anatomical axes: X forward toward the toes, Y medial, Z up. Values are the
        mean over each 20 Hz update; every sample is stored at the full 100 Hz.
      </p>

      {!units && (
        <p className="error">
          Device configuration not loaded yet — values below are raw sensor counts.
        </p>
      )}

      <div className="panel">
        <div className="chart-title">
          <span className="name">Acceleration magnitude</span>
          <span className="legend">
            <span className="sensor foot">
              <span className="name">foot</span>
            </span>
            <span className="sensor shank">
              <span className="name">shank</span>
            </span>
          </span>
        </div>
        <p className="hint">
          Reads 1.00 g on a still sensor. A different value means the scale or the
          mounting is wrong, not that the patient moved.
        </p>
        <Strip
          ring={rings.magnitude}
          unit={accelUnit}
          range={units ? MAGNITUDE_RANGE : undefined}
          series={MAGNITUDE_SERIES}
        />
        <div className="readouts" style={{ marginTop: 12 }}>
          <Axis label="foot |a|" value={tick?.foot_accel_magnitude_g ?? null} unit={accelUnit} />
          <Axis label="shank |a|" value={tick?.shank_accel_magnitude_g ?? null} unit={accelUnit} />
          <Axis label="frame" value={tick?.frame_index ?? null} unit="" />
          <Axis
            label="device time"
            value={tick ? tick.device_time_us / 1e6 : null}
            unit="s"
          />
        </div>
      </div>

      <div className="panel">
        <div className="chart-title">
          <span className="name">
            <span className="sensor foot">
              <span className="name">Foot angular rate</span>
            </span>
          </span>
          <span className="legend">
            <span>X</span>
            <span>Y</span>
            <span>Z</span>
          </span>
        </div>
        <Strip
          ring={rings.footGyro}
          unit={gyroUnit}
          series={FOOT_AXES}
        />
        <div className="readouts" style={{ marginTop: 12 }}>
          <Axis label="foot ax" value={tick?.foot_accel_g[0] ?? null} unit={accelUnit} />
          <Axis label="foot ay" value={tick?.foot_accel_g[1] ?? null} unit={accelUnit} />
          <Axis label="foot az" value={tick?.foot_accel_g[2] ?? null} unit={accelUnit} />
          <Axis label="foot gx" value={tick?.foot_gyro_dps[0] ?? null} unit={gyroUnit} />
          <Axis label="foot gy" value={tick?.foot_gyro_dps[1] ?? null} unit={gyroUnit} />
          <Axis label="foot gz" value={tick?.foot_gyro_dps[2] ?? null} unit={gyroUnit} />
        </div>
      </div>

      <div className="panel">
        <div className="chart-title">
          <span className="name">
            <span className="sensor shank">
              <span className="name">Shank angular rate</span>
            </span>
          </span>
          <span className="legend">
            <span>X</span>
            <span>Y</span>
            <span>Z</span>
          </span>
        </div>
        <Strip
          ring={rings.shankGyro}
          unit={gyroUnit}
          series={SHANK_AXES}
        />
        <div className="readouts" style={{ marginTop: 12 }}>
          <Axis label="shank ax" value={tick?.shank_accel_g[0] ?? null} unit={accelUnit} />
          <Axis label="shank ay" value={tick?.shank_accel_g[1] ?? null} unit={accelUnit} />
          <Axis label="shank az" value={tick?.shank_accel_g[2] ?? null} unit={accelUnit} />
          <Axis label="shank gx" value={tick?.shank_gyro_dps[0] ?? null} unit={gyroUnit} />
          <Axis label="shank gy" value={tick?.shank_gyro_dps[1] ?? null} unit={gyroUnit} />
          <Axis label="shank gz" value={tick?.shank_gyro_dps[2] ?? null} unit={gyroUnit} />
        </div>
      </div>

      <div className="panel">
        <h2>Gait</h2>
        <p className="hint">
          What the device's detector is doing right now. Cycles are counted from
          boot; the Cycles view has the measurements for a recorded session.
        </p>
        <div className="readouts">
          <div className="readout">
            <span className="label">Gait state</span>
            <span className="value" style={{ fontSize: 15 }}>
              {snapshot?.status
                ? (vocabulary?.gait_states[snapshot.status.gait_state] ?? "unknown")
                : <span className="absent">—</span>}
            </span>
          </div>
          <Axis label="cycles" value={snapshot?.status?.cycles_completed ?? null} unit="" />
          <div className="readout">
            <span className="label">Zero velocity</span>
            <span className="value" style={{ fontSize: 15 }}>
              {snapshot?.status?.gait_state === 4 ? "holding" : (
                <span className="absent">no</span>
              )}
            </span>
          </div>
        </div>
      </div>

      <Orientation device={device} />

      <div className="panel">
        <h2>Frame quality</h2>
        <p className="hint">
          Flags raised anywhere in the last update. A repeated shank sample is
          expected about once every seven seconds: the two sensors run on
          independent clocks.
        </p>
        <div className="readouts">
          <div className="readout wide">
            <span className="label">Flags in the last update</span>
            <span className="value" style={{ fontSize: 14 }}>
              {flags.length === 0 ? (
                <span className="absent">none</span>
              ) : (
                flags.join(", ")
              )}
            </span>
          </div>
          <Axis label="dropped" value={snapshot?.status?.frames_dropped ?? null} unit="" />
          <Axis label="I²C errors" value={snapshot?.status?.i2c_errors ?? null} unit="" />
          <Axis label="shank repeats" value={snapshot?.status?.shank_repeated ?? null} unit="" />
          <Axis label="missing messages" value={snapshot?.missing_messages ?? null} unit="" />
        </div>
      </div>

      {config && (
        <p className="hint">
          Sample rate {config.imu.sample_hz} Hz · accelerometer ±{config.imu.accel_range_g} g ·
          gyroscope ±{config.imu.gyro_range_dps} °/s
        </p>
      )}
    </>
  );
}
