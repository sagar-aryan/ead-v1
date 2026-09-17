/**
 * Static calibration: five seconds of stillness, measured on the device.
 *
 * The work happens in the firmware — it has every sample, and the orientation
 * estimator that will use the record runs there too. This panel starts the
 * window, follows it through STATUS, and shows what came back. The numbers are
 * shown rather than summarised into a verdict: a gyro bias and a mounting tilt
 * are what a reader needs to judge whether the device is behaving.
 */
import { useState } from "react";

import { api, type CalibrationSensor } from "../api";
import type { DeviceApi } from "../useDevice";

/** The documented default; the device accepts 2–30 s. */
const DURATION_MS = 5000;

const STATES = ["not calibrated", "collecting", "ready", "rejected"];

const vector = (v: number[], digits: number) =>
  v.map((n) => (n >= 0 ? "+" : "") + n.toFixed(digits)).join("  ");

function SensorRow({ name, sensor }: { name: "foot" | "shank"; sensor: CalibrationSensor }) {
  return (
    <tr>
      <td>
        <span className={`sensor ${name}`}>
          <span className="name">{name}</span>
        </span>
      </td>
      <td className="num">{vector(sensor.gyro_bias_dps, 2)}</td>
      <td className="num">{sensor.tilt_deg.toFixed(1)}°</td>
      <td className="num">{sensor.accel_magnitude_g.toFixed(3)}</td>
      <td className="num">{sensor.gyro_std_dps.toFixed(2)}</td>
    </tr>
  );
}

export function Calibration({ device }: { device: DeviceApi }) {
  const [error, setError] = useState<string | null>(null);
  const status = device.snapshot?.status ?? null;
  const record = device.snapshot?.calibration ?? null;
  const rejections = device.snapshot?.calibration_rejections ?? [];
  const state = status?.calibration_state ?? 0;
  const collecting = state === 1;
  const wanted = Math.round((DURATION_MS / 1000) * 100);

  const run = async (start: boolean) => {
    try {
      await (start ? api.startCalibration(DURATION_MS) : api.cancelCalibration());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="panel">
      <h2>Calibration</h2>
      <p className="hint">
        Stand still for five seconds. The device measures each gyroscope's
        resting rate, which would otherwise integrate into drift, and the
        direction of gravity, which is how a strapped-on board's tilt is removed.
        The record lives in the device's memory and is lost when it resets.
      </p>

      {!device.connected && <p className="empty">Connect the device to calibrate.</p>}

      {device.connected && (
        <>
          <div className="row">
            <button className="primary" onClick={() => run(true)} disabled={collecting}>
              {record ? "Calibrate again" : "Start calibration"}
            </button>
            {collecting && <button onClick={() => run(false)}>Cancel</button>}
            <div className="field">
              <label>State</label>
              <span className="num">
                {STATES[state] ?? "unknown"}
                {collecting && ` — ${status?.calibration_samples ?? 0} of ${wanted} frames`}
              </span>
            </div>
          </div>
          {error && <p className="error">{error}</p>}

          {rejections.length > 0 && (
            <p className="error">Rejected: {rejections.join(", ")}. Hold still and try again.</p>
          )}

          {record && (
            <>
              <table style={{ marginTop: 12 }}>
                <thead>
                  <tr>
                    <th style={{ width: 90 }}>Sensor</th>
                    <th>Gyro bias (°/s, chip X Y Z)</th>
                    <th style={{ width: 90 }}>Tilt</th>
                    <th style={{ width: 90 }}>|a| (g)</th>
                    <th style={{ width: 110 }}>Gyro σ (°/s)</th>
                  </tr>
                </thead>
                <tbody>
                  <SensorRow name="foot" sensor={record.foot} />
                  <SensorRow name="shank" sensor={record.shank} />
                </tbody>
              </table>
              <p className="hint" style={{ marginTop: 10 }}>
                {record.samples.toLocaleString()} frames. |a| near 1.000 confirms the
                scale factor as well as the stillness; tilt is how far the board
                sits from upright and is removed from every measurement that
                follows.
              </p>
            </>
          )}
        </>
      )}
    </div>
  );
}
