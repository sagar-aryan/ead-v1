/**
 * Guided mounting check: three short moves that prove each sensor is mounted the
 * way the firmware's mount map assumes.
 *
 * It reads the live stream already running, so it needs nothing from the device
 * beyond a connection and a known configuration. The verdicts and thresholds are
 * in `mounting.ts`, tested there; this file collects samples and shows numbers.
 */
import { useEffect, useRef, useState } from "react";

import type { LiveTick } from "../api";
import {
  type RotationVerdict,
  type StillVerdict,
  type Vec3,
  rotationVerdict,
  stillVerdict,
} from "../mounting";
import type { DeviceApi, Ring } from "../useDevice";

interface Step {
  id: string;
  title: string;
  instruction: string;
  seconds: number;
  /** Which sensor moves; the still step checks both. */
  sensor: "foot" | "shank" | "both";
}

// Raising the toes and extending the knee both rotate the segment about the
// medial axis in the negative sense, so both expect a negative Y rate.
const EXPECTED_AXIS = 1;
const EXPECTED_SIGN = -1;

const STEPS: Step[] = [
  {
    id: "still",
    title: "Stand still",
    instruction: "Stand upright with your weight on both feet and hold still.",
    seconds: 3,
    sensor: "both",
  },
  {
    id: "toes",
    title: "Raise your toes",
    instruction:
      "Keep your heel on the floor. Lift your toes as far as they go, then lower them.",
    seconds: 4,
    sensor: "foot",
  },
  {
    id: "knee",
    title: "Extend your knee",
    instruction: "Sit down. Straighten your knee to lift your foot forward, then lower it.",
    seconds: 4,
    sensor: "shank",
  },
];

type Result =
  | { kind: "still"; foot: StillVerdict; shank: StillVerdict }
  | { kind: "rotation"; sensor: "foot" | "shank"; verdict: RotationVerdict };

const passed = (result: Result) =>
  result.kind === "still" ? result.foot.pass && result.shank.pass : result.verdict.pass;

/**
 * Samples written to a ring after `from` on its clock.
 *
 * The rotation steps read the rings rather than the tick state: ticks reach
 * React at 5 Hz for the readouts, which is too slow to catch the peak of a
 * deliberate movement, while the rings hold every 20 Hz update.
 */
function since(ring: Ring, from: number): Vec3[] {
  const out: Vec3[] = [];
  for (let i = 0; i < ring.length; i++) {
    if (ring.time[i] > from) {
      out.push([ring.values[0][i], ring.values[1][i], ring.values[2][i]]);
    }
  }
  return out;
}

const latest = (ring: Ring) => (ring.length ? ring.time[ring.length - 1] : 0);

const vector = (v: Vec3, unit: string) =>
  `${v.map((n) => (n >= 0 ? "+" : "") + n.toFixed(unit === "g" ? 2 : 0)).join("  ")} ${unit}`;

export function MountingCheck({ device }: { device: DeviceApi }) {
  const [step, setStep] = useState<Step | null>(null);
  const [remaining, setRemaining] = useState(0);
  const [results, setResults] = useState<Record<string, Result>>({});
  const samples = useRef<LiveTick[]>([]);
  /** Ring clock reading when the step started, so only its samples are read. */
  const from = useRef(0);

  const tick = device.tick;
  const ready = device.connected && tick !== null && tick.anatomical;

  // Accelerations come from the tick state: the still step wants a mean, and
  // 5 Hz over three seconds is plenty for that.
  useEffect(() => {
    if (step && tick) samples.current.push(tick);
  }, [tick, step]);

  useEffect(() => {
    if (!step) return;
    const started = Date.now();
    const timer = setInterval(() => {
      const left = step.seconds - (Date.now() - started) / 1000;
      if (left > 0) {
        setRemaining(left);
        return;
      }
      clearInterval(timer);
      const collected = samples.current;
      const accel = (sensor: "foot" | "shank"): Vec3[] =>
        collected.map((s) => (sensor === "foot" ? s.foot_accel_g : s.shank_accel_g));
      const gyro = (sensor: "foot" | "shank"): Vec3[] =>
        since(sensor === "foot" ? device.rings.footGyro : device.rings.shankGyro, from.current);
      const result: Result =
        step.sensor === "both"
          ? { kind: "still", foot: stillVerdict(accel("foot")), shank: stillVerdict(accel("shank")) }
          : {
              kind: "rotation",
              sensor: step.sensor,
              verdict: rotationVerdict(gyro(step.sensor), EXPECTED_AXIS, EXPECTED_SIGN),
            };
      setResults((current) => ({ ...current, [step.id]: result }));
      setStep(null);
      setRemaining(0);
    }, 100);
    return () => clearInterval(timer);
  }, [step, device.rings]);

  const start = (next: Step) => {
    samples.current = [];
    from.current = Math.max(latest(device.rings.footGyro), latest(device.rings.shankGyro));
    setRemaining(next.seconds);
    setStep(next);
  };

  const done = STEPS.filter((s) => results[s.id]);
  const allPass = done.length === STEPS.length && done.every((s) => passed(results[s.id]));

  return (
    <div className="panel">
      <h2>Mounting check</h2>
      <p className="hint">
        Three moves that confirm each sensor sits on the leg the way the firmware
        assumes. Standing still must read +Z on both; raising the toes must turn
        the foot about its Y axis, and extending the knee must turn the shank
        about its Y axis, both negative.
      </p>

      {!ready && (
        <p className="empty">
          {device.connected
            ? "Waiting for the device configuration."
            : "Connect the device to run the check."}
        </p>
      )}

      {ready && (
        <table>
          <thead>
            <tr>
              <th style={{ width: "34%" }}>Step</th>
              <th>Measured</th>
              <th style={{ width: 110 }}>Result</th>
              <th style={{ width: 96 }} />
            </tr>
          </thead>
          <tbody>
            {STEPS.map((s) => {
              const result = results[s.id];
              const running = step?.id === s.id;
              return (
                <tr key={s.id}>
                  <td>
                    <div>{s.title}</div>
                    <div className="hint" style={{ margin: 0 }}>
                      {s.instruction}
                    </div>
                  </td>
                  <td className="num">
                    {running && `hold — ${remaining.toFixed(1)} s`}
                    {!running && result?.kind === "still" && (
                      <>
                        <div>
                          <span className="sensor foot">
                            <span className="name">foot</span>
                          </span>{" "}
                          {vector(result.foot.mean, "g")}
                        </div>
                        <div>
                          <span className="sensor shank">
                            <span className="name">shank</span>
                          </span>{" "}
                          {vector(result.shank.mean, "g")}
                        </div>
                      </>
                    )}
                    {!running && result?.kind === "rotation" && (
                      <div>
                        peak {vector(result.verdict.peak, "°/s")} — {result.verdict.reason}
                      </div>
                    )}
                    {!running && !result && <span className="absent">not run</span>}
                  </td>
                  <td>
                    {result ? (
                      <span className={passed(result) ? "verdict pass" : "verdict fail"}>
                        {passed(result) ? "PASS" : "FAIL"}
                      </span>
                    ) : (
                      <span className="absent">—</span>
                    )}
                  </td>
                  <td>
                    <button onClick={() => start(s)} disabled={step !== null}>
                      {result ? "Again" : "Start"}
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      {ready && results.still?.kind === "still" && !results.still.foot.pass && (
        <p className="hint">Foot: {results.still.foot.reason}</p>
      )}
      {ready && results.still?.kind === "still" && !results.still.shank.pass && (
        <p className="hint">Shank: {results.still.shank.reason}</p>
      )}
      {ready && allPass && (
        <p className="hint">
          Both sensors match their mount maps. Record this against PROB-002.
        </p>
      )}
    </div>
  );
}
