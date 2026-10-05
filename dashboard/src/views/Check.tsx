/**
 * CHECK (`ead --check`): every line to both BNO086 sensors, and each motor on
 * its own (doc 07 §7, doc 11 §6, DEC-018). Not available while a session
 * records; the device refuses too.
 */
import { useCallback, useEffect, useState } from "react";

import { api, type SensorCheckReport, type ServiceTest } from "../api";
import type { DeviceApi } from "../useDevice";

/** The check steps in the order the device runs them, and what each proves. */
const CHECK_STEPS: [string, string][] = [
  ["int_high_in_reset", "INT is not shorted to ground"],
  ["booted", "3V3, GND, RST and INT: the sensor starts after a reset"],
  ["read_valid", "SCK, MISO and CS: a packet reads back"],
  ["int_released", "CS and INT reach the same board"],
  ["wake", "WAKE: the sensor answers it"],
  ["product_id", "MOSI: the sensor answers a request"],
  ["reports", "Accelerometer and gyroscope stream"],
];

/** The device's duty range for any motor drive: doc 06's 20 % floor, and up to
 * 100 % since DEC-025 (the contract capped it at 80 %). */
const MIN_PERCENT = 20;
const MAX_PERCENT = 100;

function Verdict({ pass }: { pass: boolean }) {
  return <span className={pass ? "verdict pass" : "verdict fail"}>{pass ? "PASS" : "FAIL"}</span>;
}

function rate(v: [number, number, number] | undefined): string {
  return v ? Math.hypot(...v).toFixed(0) : "—";
}

interface PulseAnswer {
  testId: number;
  felt: boolean | null;
}

export function Check({ device, recording }: { device: DeviceApi; recording: string | null }) {
  const { snapshot, config, tick, connected } = device;
  const [report, setReport] = useState<SensorCheckReport | null>(null);
  const [checking, setChecking] = useState(false);
  const [percent, setPercent] = useState(50);
  const [pulsing, setPulsing] = useState<number | null>(null);
  const [answers, setAnswers] = useState<Record<number, PulseAnswer>>({});
  const [history, setHistory] = useState<ServiceTest[]>([]);
  // Each panel shows its own failure where the button was pressed.
  const [checkError, setCheckError] = useState<string | null>(null);
  const [motorError, setMotorError] = useState<string | null>(null);

  const blocked = recording !== null;

  const runCheck = useCallback(async (rerun: boolean) => {
    setChecking(true);
    setCheckError(null);
    try {
      setReport(await api.sensorCheck(rerun));
    } catch (e) {
      setCheckError(String(e));
    } finally {
      setChecking(false);
    }
  }, []);

  // The check from this boot, as soon as the link is up.
  useEffect(() => {
    if (connected && !blocked && report === null) runCheck(false);
  }, [connected, blocked, report, runCheck]);

  useEffect(() => {
    api.serviceTests().then(setHistory).catch(() => undefined);
  }, [answers]);

  const pulse = async (motor: number) => {
    setPulsing(motor);
    setMotorError(null);
    try {
      const duty = Math.round((percent * 255) / 100);
      const testId = await api.motorPulse(motor, duty);
      setAnswers((a) => ({ ...a, [motor]: { testId, felt: null } }));
    } catch (e) {
      setMotorError(String(e));
    } finally {
      setPulsing(null);
    }
  };

  const answer = async (motor: number, felt: boolean) => {
    const current = answers[motor];
    if (!current) return;
    try {
      await api.recordMotorFelt(current.testId, felt);
      setAnswers((a) => ({ ...a, [motor]: { ...current, felt } }));
    } catch (e) {
      setMotorError(String(e));
    }
  };

  /** The latest recorded answer for a motor, from the log. */
  const lastLogged = (motor: number) =>
    history.find((t) => t.kind === "motor_pulse" && t.motor === motor && t.felt !== null);

  const motors = config?.pins.motors ?? [];
  const degrees = config?.haptics.motor_positions_deg ?? [];

  return (
    <>
      <h1>Check</h1>
      <p className="page-hint">
        Every line to both sensors, and each motor on its own. Nothing here is recorded with a
        patient session; motor pulses and re-run checks are kept in the service-test log.
      </p>

      {!connected && <p className="empty">Connect the device on the Device page first.</p>}
      {blocked && <p className="empty">Not available while session {recording} is recording.</p>}

      {connected && (
        <div className="panel">
          <h2>Sensors</h2>
          <p className="hint">
            A table with every step passing proves every wire. A failure names the step: on the
            shared lines (SCK, MOSI, MISO, RST, WAKE) it says which sensor's cable, not always which
            single wire, and 3V3 cannot be told apart from GND. Running the check again resets both
            sensors; the data stream pauses for about two seconds.
          </p>
          <div className="button-group">
            <button className="primary" disabled={checking || blocked} onClick={() => runCheck(true)}>
              {checking ? "Checking…" : "Run the check again"}
            </button>
          </div>
          {checkError && <p className="error">{checkError}</p>}
          {report && (
            <table style={{ marginTop: 16 }}>
              <thead>
                <tr>
                  <th>Step</th>
                  <th>What it proves</th>
                  <th>
                    <span className="sensor foot">
                      <span className="name">Foot</span>
                    </span>
                  </th>
                  <th>
                    <span className="sensor shank">
                      <span className="name">Shank</span>
                    </span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {CHECK_STEPS.map(([step, proves]) => (
                  <tr key={step}>
                    <td className="num">{step}</td>
                    <td>{proves}</td>
                    <td>
                      <Verdict pass={report.foot.passed.includes(step)} />
                    </td>
                    <td>
                      <Verdict pass={report.shank.passed.includes(step)} />
                    </td>
                  </tr>
                ))}
                <tr>
                  <td className="num">identity</td>
                  <td>Part, firmware version, build</td>
                  {[report.foot, report.shank].map((s, i) => (
                    <td key={i} className="num">
                      {s.part_number ? `${s.part_number} v${s.version} b${s.build_number}` : "—"}
                    </td>
                  ))}
                </tr>
                <tr>
                  <td className="num">timing</td>
                  <td>Reset to INT; WAKE to INT</td>
                  {[report.foot, report.shank].map((s, i) => (
                    <td key={i} className="num">
                      {s.boot_ms ?? "—"} ms; {s.wake_us ?? "—"} µs
                    </td>
                  ))}
                </tr>
              </tbody>
            </table>
          )}
        </div>
      )}

      {connected && (
        <div className="panel">
          <h2>Which sensor is which</h2>
          <p className="hint">
            Turn the sensor strapped to the foot by hand: its rate rises here and the shank's
            stays near zero. If the shank row moves instead, the two cables are swapped.
          </p>
          <div className="readouts">
            <div className="readout">
              <span className="label">Foot rotation rate</span>
              <span className="value num">
                {rate(tick?.foot_gyro_dps)}
                <span className="unit">°/s</span>
              </span>
            </div>
            <div className="readout">
              <span className="label">Shank rotation rate</span>
              <span className="value num">
                {rate(tick?.shank_gyro_dps)}
                <span className="unit">°/s</span>
              </span>
            </div>
          </div>
        </div>
      )}

      {connected && (
        <div className="panel">
          <h2>Motors</h2>
          {!snapshot?.motor_service_test ? (
            <p className="empty">This firmware cannot pulse the motors.</p>
          ) : (
            <>
              <p className="hint">
                One motor at a time, for one second, at the strength set here. The device ends each
                pulse by itself and enforces its limits: {MIN_PERCENT}–{MAX_PERCENT} %
                duty, at most 5 s on per motor in any 10 s. The device cannot sense a motor turning,
                so say whether you felt it.
              </p>
              <div className="field" style={{ maxWidth: 360 }}>
                <label htmlFor="motor-strength">Strength: {percent} %</label>
                <input
                  id="motor-strength"
                  type="range"
                  min={MIN_PERCENT}
                  max={MAX_PERCENT}
                  step={5}
                  value={percent}
                  onChange={(e) => setPercent(Number(e.target.value))}
                />
              </div>
              {motorError && <p className="error">{motorError}</p>}
              <table style={{ marginTop: 16 }}>
                <thead>
                  <tr>
                    <th>Motor</th>
                    <th>Position</th>
                    <th>GPIO</th>
                    <th />
                    <th>Felt?</th>
                    <th>Last recorded</th>
                  </tr>
                </thead>
                <tbody>
                  {motors.map((gpio, i) => {
                    const motor = i + 1;
                    const current = answers[motor];
                    const logged = lastLogged(motor);
                    return (
                      <tr key={motor}>
                        <td className="num">M{motor}</td>
                        <td className="num">{degrees[i]}°</td>
                        <td className="num">{gpio}</td>
                        <td>
                          <button
                            disabled={blocked || pulsing !== null}
                            onClick={() => pulse(motor)}
                          >
                            {pulsing === motor ? "Pulsing…" : "Pulse 1 s"}
                          </button>
                        </td>
                        <td>
                          {current && current.felt === null && (
                            <span className="button-group">
                              <button onClick={() => answer(motor, true)}>Felt it</button>
                              <button onClick={() => answer(motor, false)}>Did not feel it</button>
                            </span>
                          )}
                          {current && current.felt !== null && (
                            <span className={current.felt ? "verdict pass" : "verdict fail"}>
                              {current.felt ? "felt" : "not felt"}
                            </span>
                          )}
                        </td>
                        <td className="num">
                          {logged ? (
                            `${logged.felt ? "felt" : "not felt"} at ${Math.round(((logged.duty ?? 0) * 100) / 255)} %, ${logged.at}`
                          ) : (
                            <span className="absent">none</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </>
          )}
        </div>
      )}
    </>
  );
}
