/**
 * CYCLES: every gait cycle the device measured in a session, and the trend of
 * each measurement across the session.
 *
 * A gait cycle here is right initial contact to the next right initial contact
 * (doc 05 §1) — the leg is instrumented on one side, so this is not a bilateral
 * step time and is never called one.
 *
 * Distance and speed depend on the zero-velocity windows the detector found, so
 * a cycle with poor ZUPT quality shows them as low-confidence rather than as a
 * measurement (doc 05 §8). Invalid cycles — outside the 0.45–3.00 s guards — are
 * shown, greyed, rather than hidden: a detector that silently drops what it
 * cannot explain is a detector you cannot debug.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import uPlot from "uplot";

import { api, type Cycle, type Session } from "../api";

/** Below this, doc 05 §8 says to report distance and speed as low-confidence. */
const ZUPT_ADEQUATE = 0.15;

const TRENDS: { key: keyof Cycle; label: string; unit: string; digits: number }[] = [
  { key: "cadence_steps_per_min", label: "Cadence", unit: "steps/min", digits: 0 },
  { key: "cycle_time_s", label: "Cycle time", unit: "s", digits: 2 },
  { key: "stance_ratio", label: "Stance ratio", unit: "", digits: 2 },
  { key: "distance_m", label: "Cycle distance", unit: "m", digits: 2 },
  { key: "speed_mps", label: "Speed", unit: "m/s", digits: 2 },
  { key: "peak_dorsiflexion_deg", label: "Peak dorsiflexion", unit: "°", digits: 1 },
  { key: "peak_shank_rate_dps", label: "Peak shank rate", unit: "°/s", digits: 0 },
  { key: "zupt_quality", label: "ZUPT quality", unit: "", digits: 2 },
];

const INK = "#15181c";
const AXIS = {
  stroke: "#868c93",
  grid: { stroke: "#eceeed", width: 1 },
  ticks: { stroke: "#e3e4e2", width: 1 },
  font: '11px "IBM Plex Mono", monospace',
};

/** One measurement across the session's valid cycles. */
function Trend({ cycles, index }: { cycles: Cycle[]; index: number }) {
  const host = useRef<HTMLDivElement>(null);
  const trend = TRENDS[index];

  useEffect(() => {
    if (!host.current) return;
    const valid = cycles.filter((c) => c.valid);
    const x = valid.map((_, i) => i + 1);
    const y = valid.map((c) => c[trend.key] as number);
    const plot = new uPlot(
      {
        width: host.current.clientWidth || 400,
        height: 130,
        legend: { show: false },
        cursor: { drag: { x: false, y: false } },
        scales: { x: { time: false } },
        series: [
          { label: "cycle" },
          { label: trend.label, stroke: INK, width: 1.5, points: { show: valid.length < 40 } },
        ],
        axes: [AXIS, { ...AXIS, size: 52 }],
      },
      [x, y] as unknown as uPlot.AlignedData,
      host.current,
    );
    const resize = () => host.current && plot.setSize({ width: host.current.clientWidth, height: 130 });
    const observer = new ResizeObserver(resize);
    observer.observe(host.current);
    return () => {
      observer.disconnect();
      plot.destroy();
    };
  }, [cycles, trend]);

  return (
    <div>
      <div className="chart-title">
        <span className="name">
          {trend.label}
          {trend.unit && ` (${trend.unit})`}
        </span>
      </div>
      <div ref={host} style={{ width: "100%", height: 130 }} />
    </div>
  );
}

function summary(values: number[]) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)];
  // Median absolute deviation: the spread measure the spec uses everywhere,
  // because one odd cycle should not move it (doc 05 §3).
  const mad = [...values].map((v) => Math.abs(v - median)).sort((a, b) => a - b)[
    Math.floor(values.length / 2)
  ];
  return { median, mad };
}

export function Cycles() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState("");
  const [cycles, setCycles] = useState<Cycle[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.sessions().then(setSessions).catch((e) => setError(String(e)));
  }, []);

  const load = useCallback(async (sessionId: string) => {
    if (!sessionId) {
      setCycles([]);
      return;
    }
    try {
      setCycles(await api.cycles(sessionId));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    load(selected);
  }, [selected, load]);

  const valid = useMemo(() => cycles.filter((c) => c.valid), [cycles]);
  const measured = useMemo(
    () => valid.filter((c) => c.zupt_quality >= ZUPT_ADEQUATE),
    [valid],
  );
  const cadence = summary(valid.map((c) => c.cadence_steps_per_min));
  const distance = measured.reduce((total, c) => total + c.distance_m, 0);
  const walkingTime = measured.reduce((total, c) => total + c.cycle_time_s, 0);

  return (
    <>
      <h1>Cycles</h1>
      <p className="page-hint">
        One row per gait cycle: right initial contact to the next. Only the right
        leg is instrumented, so these are not bilateral step times.
      </p>

      <div className="panel">
        <div className="row">
          <div className="field">
            <label htmlFor="session">Session</label>
            <select id="session" value={selected} onChange={(e) => setSelected(e.target.value)}>
              <option value="">Select…</option>
              {sessions.map((s) => (
                <option key={s.session_id} value={s.session_id}>
                  {s.session_id} — {s.patient_name}
                </option>
              ))}
            </select>
          </div>
        </div>
        {error && <p className="error">{error}</p>}
      </div>

      {!selected && <p className="empty">Select a recorded session.</p>}
      {selected && cycles.length === 0 && (
        <p className="empty">
          No cycles in this session. The device only detects them once it has been
          calibrated and the patient has walked.
        </p>
      )}

      {cycles.length > 0 && (
        <>
          <div className="panel">
            <div className="readouts">
              <div className="readout">
                <span className="label">Cycles</span>
                <span className="value num">{valid.length}</span>
              </div>
              <div className="readout">
                <span className="label">Rejected</span>
                <span className="value num">{cycles.length - valid.length}</span>
              </div>
              <div className="readout">
                <span className="label">Cadence median</span>
                <span className="value num">
                  {cadence ? cadence.median.toFixed(0) : "—"}
                  <span className="unit">steps/min</span>
                </span>
              </div>
              <div className="readout">
                <span className="label">Cadence MAD</span>
                <span className="value num">{cadence ? cadence.mad.toFixed(1) : "—"}</span>
              </div>
              <div className="readout">
                <span className="label">Distance</span>
                <span className="value num">
                  {distance.toFixed(2)}
                  <span className="unit">m</span>
                </span>
              </div>
              <div className="readout">
                <span className="label">Mean speed</span>
                <span className="value num">
                  {walkingTime > 0 ? (distance / walkingTime).toFixed(2) : "—"}
                  <span className="unit">m/s</span>
                </span>
              </div>
            </div>
            <p className="hint" style={{ marginTop: 10 }}>
              Distance and speed sum only the {measured.length} of {valid.length} cycles
              whose zero-velocity quality reaches {ZUPT_ADEQUATE.toFixed(2)}; the rest are
              shown per cycle but not counted, rather than corrected (doc 05 §8).
            </p>
          </div>

          <div className="panel">
            <h2>Per cycle</h2>
            <p className="hint">
              Rejected cycles are greyed: they fell outside the 0.45–3.00 s guards,
              which usually means a contact was missed or doubled.
            </p>
            <div style={{ overflowX: "auto" }}>
              <table>
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Time (s)</th>
                    <th>Cycle (s)</th>
                    <th>Stance</th>
                    <th>Cadence</th>
                    <th>Distance (m)</th>
                    <th>Speed (m/s)</th>
                    <th>ZUPT</th>
                    <th>Dorsi (°)</th>
                    <th>Contact (°)</th>
                    <th>Shank (°/s)</th>
                  </tr>
                </thead>
                <tbody>
                  {cycles.map((c, index) => (
                    <tr key={c.start_frame} style={c.valid ? undefined : { color: "#868c93" }}>
                      <td className="num">{index + 1}</td>
                      <td className="num">
                        {((c.start_us - cycles[0].start_us) / 1e6).toFixed(1)}
                      </td>
                      <td className="num">{c.cycle_time_s.toFixed(2)}</td>
                      <td className="num">{c.stance_ratio.toFixed(2)}</td>
                      <td className="num">{c.cadence_steps_per_min.toFixed(0)}</td>
                      <td className="num">
                        {c.zupt_quality >= ZUPT_ADEQUATE ? (
                          c.distance_m.toFixed(2)
                        ) : (
                          <span className="absent" style={{ fontSize: 13 }}>
                            {c.distance_m.toFixed(2)} low
                          </span>
                        )}
                      </td>
                      <td className="num">{c.speed_mps.toFixed(2)}</td>
                      <td className="num">{c.zupt_quality.toFixed(2)}</td>
                      <td className="num">{c.peak_dorsiflexion_deg.toFixed(1)}</td>
                      <td className="num">{c.contact_sagittal_deg.toFixed(1)}</td>
                      <td className="num">{c.peak_shank_rate_dps.toFixed(0)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div className="panel">
            <h2>Trends</h2>
            <p className="hint">
              Each measurement across the {valid.length} valid cycles, in order. A
              trend that wanders is either the patient or the detector; the raw
              view is where that gets settled.
            </p>
            <div className="trend-grid">
              {TRENDS.map((trend, index) => (
                <Trend key={trend.key} cycles={cycles} index={index} />
              ))}
            </div>
          </div>
        </>
      )}
    </>
  );
}
