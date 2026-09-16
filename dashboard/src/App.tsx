import { useEffect, useRef, useState } from "react";
import type {
  Cycle,
  DeviceState,
  GaitEvent,
  HapticRecord,
  LiveSnapshot,
  ReferenceProfile,
  Segment,
  Session,
  WsStatus,
} from "./types";

// ESP32-S3 local AP endpoint (doc 08 §1). Placeholder until device is live.
const WS_URL = "ws://192.168.4.1:8080/ws";

type View =
  | "LIVE"
  | "TRENDS"
  | "CYCLES"
  | "RAW"
  | "EVENTS"
  | "HAPTICS"
  | "REFERENCES"
  | "SESSIONS"
  | "EXPORT";

const VIEWS: View[] = [
  "LIVE",
  "TRENDS",
  "CYCLES",
  "RAW",
  "EVENTS",
  "HAPTICS",
  "REFERENCES",
  "SESSIONS",
  "EXPORT",
];

const DEVICE_STATES: DeviceState[] = [
  "disconnected",
  "booting",
  "calibrating",
  "reference",
  "ready",
  "running",
  "paused",
  "fault",
  "recovered",
];

// Seven primary trend panels (doc 11 §TRENDS) — exact order matters.
const TREND_PANELS = [
  "Cadence",
  "Unilateral cycle symmetry proxy",
  "Error score",
  "Stance/swing",
  "Foot angle",
  "Haptic response",
  "ZUPT quality",
] as const;

// --- Placeholder demo data (replaced by real WS/store data in later tasks) ---
const demoCycles: Cycle[] = [
  {
    session_id: "20260101-000000-ab12",
    segment_id: 1,
    cycle_id: 1,
    start_us: 0,
    end_us: 1100000,
    cycle_time_s: 1.1,
    stance_s: 0.68,
    swing_s: 0.42,
    cadence_spm: 109,
    cycle_distance_m: 1.2,
    speed_mps: 1.09,
    unilateral_symmetry_proxy: 0.94,
    error_score: 0.12,
    confidence: 0.88,
    primary_error_class: "none",
    zupt_quality: 0.95,
  },
];

const demoEvents: GaitEvent[] = [
  {
    session_id: "20260101-000000-ab12",
    segment_id: 1,
    cycle_id: 1,
    timestamp_us: 0,
    event_type: "CYCLE_START",
    quality: 1,
  },
  {
    session_id: "20260101-000000-ab12",
    segment_id: 1,
    cycle_id: 1,
    timestamp_us: 50000,
    event_type: "INITIAL_CONTACT",
    quality: 0.97,
  },
  {
    session_id: "20260101-000000-ab12",
    segment_id: 1,
    cycle_id: 1,
    timestamp_us: 700000,
    event_type: "TOE_OFF",
    quality: 0.93,
  },
];

const demoHaptics: HapticRecord[] = [];
const demoReferences: ReferenceProfile[] = [];
const demoSessions: Session[] = [];

function DeviceBadge({ state }: { state: DeviceState }) {
  return (
    <span data-testid="device-state-badge" title="Device state">
      ● {state.toUpperCase()}
    </span>
  );
}

function WsBadge({ status }: { status: WsStatus }) {
  return (
    <span data-testid="ws-status" title={WS_URL}>
      WS:{status.toUpperCase()} ({WS_URL})
    </span>
  );
}

export default function App() {
  const [view, setView] = useState<View>("LIVE");
  const [deviceState, setDeviceState] = useState<DeviceState>("disconnected");
  const [wsStatus, setWsStatus] = useState<WsStatus>("disconnected");
  const [live] = useState<LiveSnapshot | null>(null);
  const [cycles] = useState<Cycle[]>(demoCycles);
  const [events] = useState<GaitEvent[]>(demoEvents);
  const [haptics] = useState<HapticRecord[]>(demoHaptics);
  const [references] = useState<ReferenceProfile[]>(demoReferences);
  const [sessions] = useState<Session[]>(demoSessions);
  const wsRef = useRef<WebSocket | null>(null);

  // Placeholder WS link: device AP may not exist on dev machines, so
  // failures must degrade to DISCONNECTED, never crash the UI.
  useEffect(() => {
    let cancelled = false;
    let sock: WebSocket | null = null;
    const connect = () => {
      if (cancelled) return;
      setWsStatus("connecting");
      try {
        sock = new WebSocket(WS_URL);
        wsRef.current = sock;
        sock.binaryType = "arraybuffer";
        sock.onopen = () => {
          if (!cancelled) setWsStatus("connected");
        };
        sock.onclose = () => {
          if (!cancelled) {
            setWsStatus("disconnected");
            setDeviceState("disconnected");
          }
        };
        sock.onerror = () => {
          if (!cancelled) setWsStatus("error");
        };
        // TODO: decode binary frames per doc 08 frame header
        // (u16 proto, u8 type, u8 flags, u32 len, u32 seq, u64 time_us)
        // and dispatch RAW_SAMPLE_BATCH / EVENT_BATCH / STEP_BATCH /
        // HAPTIC_BATCH / STATUS. UI renders downsampled ~20 Hz views;
        // the Rust backend preserves every raw sample.
      } catch {
        if (!cancelled) setWsStatus("error");
      }
    };
    connect();
    return () => {
      cancelled = true;
      try {
        sock?.close();
      } catch {
        /* ignore close errors on unmount */
      }
      wsRef.current = null;
    };
  }, []);

  return (
    <div style={{ fontFamily: "sans-serif", padding: 16 }}>
      <header
        style={{ display: "flex", gap: 16, alignItems: "center", flexWrap: "wrap" }}
      >
        <h1>EAD V1 Dashboard (skeleton)</h1>
        <DeviceBadge state={deviceState} />
        <WsBadge status={wsStatus} />
        {/* Manual override until STATUS frames drive the badge */}
        <select
          aria-label="device-state-override"
          value={deviceState}
          onChange={(e) => setDeviceState(e.target.value as DeviceState)}
        >
          {DEVICE_STATES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </header>

      <nav style={{ display: "flex", gap: 8, margin: "12px 0", flexWrap: "wrap" }}>
        {VIEWS.map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            aria-pressed={view === v}
            style={{ fontWeight: view === v ? "bold" : "normal" }}
          >
            {v}
          </button>
        ))}
      </nav>

      {view === "LIVE" && (
        <section>
          <h2>LIVE</h2>
          <p>
            Patient: {live?.patient_name ?? "—"} ({live?.patient_id ?? "—"}) ·
            Reference: {live?.reference_profile_id ?? "—"} · Session:{" "}
            {live?.session_id ?? "—"} / Segment {live?.segment_id ?? "—"} ·
            Elapsed: {live?.elapsed_s ?? "—"}s · Cycle{" "}
            {live?.cycle_id ?? "—"}
          </p>
          <ul>
            <li>Cadence: {live?.cadence_spm ?? "—"} spm</li>
            <li>Speed: {live?.speed_mps ?? "—"} m/s</li>
            <li>
              Stance/swing: {live?.stance_s ?? "—"}s / {live?.swing_s ?? "—"}s
            </li>
            <li>Symmetry proxy: {live?.unilateral_symmetry_proxy ?? "—"}</li>
            <li>
              Foot/shank orientation:{" "}
              {live ? JSON.stringify([live.foot_quat, live.shank_quat]) : "—"}
            </li>
            <li>
              Error: {live?.error_score ?? "—"} (conf{" "}
              {live?.confidence ?? "—"}) · {live?.error_class ?? "—"}
            </li>
            <li>
              ZUPT: {live?.zupt_state ?? "—"} (q {live?.zupt_quality ?? "—"})
            </li>
            <li>Sensor: {live?.sensor_status ?? "—"}</li>
            <li>Motor: {live?.motor_state ?? "—"}</li>
          </ul>
        </section>
      )}

      {view === "TRENDS" && (
        <section>
          <h2>TRENDS (7 panels)</h2>
          {TREND_PANELS.map((p) => (
            <div
              key={p}
              style={{ border: "1px solid #ccc", margin: "8px 0", padding: 8 }}
            >
              <h3>{p}</h3>
              <p>placeholder chart — backend downsamples to ~20 Hz for display</p>
            </div>
          ))}
        </section>
      )}

      {view === "CYCLES" && (
        <section>
          <h2>CYCLES</h2>
          <table border={1} cellPadding={4}>
            <thead>
              <tr>
                <th>cycle_id</th>
                <th>start/end</th>
                <th>cycle_time</th>
                <th>stance</th>
                <th>swing</th>
                <th>speed</th>
                <th>error</th>
                <th>conf</th>
                <th>class</th>
                <th>haptic ch/level</th>
              </tr>
            </thead>
            <tbody>
              {cycles.map((c) => (
                <tr key={c.cycle_id}>
                  <td>{c.cycle_id}</td>
                  <td>
                    {c.start_us}/{c.end_us}
                  </td>
                  <td>{c.cycle_time_s}</td>
                  <td>{c.stance_s}</td>
                  <td>{c.swing_s}</td>
                  <td>{c.speed_mps}</td>
                  <td>{c.error_score}</td>
                  <td>{c.confidence}</td>
                  <td>{c.primary_error_class}</td>
                  <td>—</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      {view === "RAW" && (
        <section>
          <h2>RAW</h2>
          <p>
            Filters (placeholder): Foot/Shank · time range · cycle · signal ·
            error markers. Full raw accel/gyro + orientation + status; error
            markers jump to the raw timestamp range.
          </p>
        </section>
      )}

      {view === "EVENTS" && (
        <section>
          <h2>EVENTS</h2>
          <ul>
            {events.map((e, i) => (
              <li key={i}>
                {e.timestamp_us} — {e.event_type} (cycle {e.cycle_id}, q{" "}
                {e.quality})
              </li>
            ))}
          </ul>
        </section>
      )}

      {view === "HAPTICS" && (
        <section>
          <h2>HAPTICS</h2>
          <p>
            Service/test UI is disabled while a patient session is RUNNING
            (doc 11 §6).
          </p>
          {haptics.length === 0 ? (
            <p>No haptic records yet.</p>
          ) : (
            <ul>
              {haptics.map((h, i) => (
                <li key={i}>
                  {h.timestamp_us} motors {h.motor_ids.join(",")} pwm {h.pwm}{" "}
                  {h.duration_ms}ms · {h.error_class}
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {view === "REFERENCES" && (
        <section>
          <h2>REFERENCES</h2>
          <p>
            Create/view/select reference profiles. A reference is immutable
            once used for an evaluation session. Finalize requires ≥30 valid
            cycles (50–100+ preferred).
          </p>
          {references.length === 0 && <p>No references yet.</p>}
        </section>
      )}

      {view === "SESSIONS" && (
        <section>
          <h2>SESSIONS (Patient → Session → Segment)</h2>
          {sessions.length === 0 ? (
            <p>No sessions yet.</p>
          ) : (
            <ul>
              {sessions.map((s) => (
                <li key={s.session_id}>
                  {s.patient_name} ({s.patient_id}) → {s.session_id}
                  <ul>
                    {s.segments.map((seg: Segment) => (
                      <li key={seg.segment_id}>
                        Segment {String(seg.segment_id).padStart(3, "0")}:{" "}
                        {seg.valid_cycle_count} cycles, {seg.error_count}{" "}
                        errors {seg.closed ? "(closed)" : "(open)"}
                      </li>
                    ))}
                  </ul>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {view === "EXPORT" && (
        <section>
          <h2>EXPORT</h2>
          <button type="button">Export CSV package</button>{" "}
          <button type="button">Export session.mat</button>{" "}
          <button type="button">Export PDF report</button>
          <p>
            Placeholder buttons — wired to Rust export commands in a later
            task (raw.csv / gait.csv / events.csv / haptics.csv /
            metadata.json, Level-5 session.mat, PDF report).
          </p>
        </section>
      )}
    </div>
  );
}
