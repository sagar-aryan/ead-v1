/**
 * CYCLES: every gait cycle the device measured in a session, and the trend of
 * each measurement across the session.
 *
 * A gait cycle here is right initial contact to the next right initial contact
 * (doc 05 §1) — the leg is instrumented on one side, so this is not a bilateral
 * step time and is never called one.
 *
 * Distance and speed depend on the zero-velocity windows the detector found, so
 * a cycle with poor ZUPT quality, or a stride or speed beyond walking, shows them
 * as not measured rather than as a measurement (doc 05 §8, `distanceMeasured`). Invalid cycles — outside the 0.45–3.00 s guards — are
 * shown, greyed, rather than hidden: a detector that silently drops what it
 * cannot explain is a detector you cannot debug.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import uPlot from "uplot";

import {
  api,
  distanceMeasured,
  sessionTitle,
  type Cycle,
  type HapticRecord,
  type Segment,
  type Session,
} from "../api";
import { className } from "./References";
import type { DeviceApi } from "../useDevice";
import type { RawFocus } from "./Raw";

/**
 * Doc 11 TRENDS: exactly seven primary panels, in its order. Each is a value per
 * valid cycle; null is "not measured", drawn as a gap, never as zero.
 */
type TrendSpec = {
  label: string;
  unit: string;
  scoredOnly?: boolean;
  value: (c: Cycle, cues: HapticRecord[]) => number | null;
};
const PRIMARY_TRENDS: TrendSpec[] = [
  { label: "Cadence", unit: "steps/min", value: (c) => c.cadence_steps_per_min },
  { label: "Unilateral cycle symmetry proxy", unit: "", value: (c) => c.symmetry_proxy },
  {
    label: "Error score",
    unit: "",
    scoredOnly: true,
    value: (c) => (c.confidence > 0 ? c.error_score : null),
  },
  { label: "Stance ratio", unit: "", value: (c) => c.stance_ratio },
  { label: "Foot angle: peak dorsiflexion", unit: "°", value: (c) => c.peak_dorsiflexion_deg },
  {
    label: "Haptic response: cue duty",
    unit: "of 255",
    scoredOnly: true,
    value: (c, cues) =>
      cues
        .filter((h) => h.cycle_start_frame === c.start_frame && h.event !== "off")
        .reduce((max, h) => Math.max(max, h.duty_a, h.duty_b), 0),
  },
  { label: "ZUPT quality", unit: "", value: (c) => c.zupt_quality },
];
const OTHER_TRENDS: TrendSpec[] = [
  { label: "Cycle time", unit: "s", value: (c) => c.cycle_time_s },
  { label: "Cycle distance", unit: "m", value: (c) => c.distance_m },
  { label: "Speed", unit: "m/s", value: (c) => c.speed_mps },
  { label: "Peak shank rate", unit: "°/s", value: (c) => c.peak_shank_rate_dps },
  {
    label: "Confidence",
    unit: "",
    scoredOnly: true,
    value: (c) => (c.confidence > 0 ? c.confidence : null),
  },
];

/**
 * Doc 06 §7. Below this the engine says its own classification should not be
 * shown, so the class is reported as undecided rather than as a finding.
 */
const CONFIDENCE_FOR_DISPLAY = 0.5;

const INK = "#15181c";
const AXIS = {
  stroke: "#868c93",
  grid: { stroke: "#eceeed", width: 1 },
  ticks: { stroke: "#e3e4e2", width: 1 },
  font: '11px "IBM Plex Mono", monospace',
};

/** One measurement across the session's valid cycles. */
function Trend({ valid, cues, trend }: { valid: Cycle[]; cues: HapticRecord[]; trend: TrendSpec }) {
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!host.current) return;
    const x = valid.map((_, i) => i + 1);
    const y = valid.map((c) => trend.value(c, cues));
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
  }, [valid, cues, trend]);

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

/** Doc 11's "haptic channels/level": the cue a cycle produced, e.g. "M5 204 · M6 120". */
function cueText(records: HapticRecord[], startFrame: number) {
  const cue = records.find((h) => h.cycle_start_frame === startFrame && h.event !== "off");
  if (!cue) return <span className="absent">—</span>;
  if (cue.duty_a === 0) {
    // DEC-027: held back while the laptop was not heard; otherwise the motor guard.
    return <span className="absent">{cue.reason === "link_lost" ? "no laptop" : "refused"}</span>;
  }
  return [[cue.motor_a, cue.duty_a], [cue.motor_b, cue.duty_b]]
    .filter(([motor]) => motor !== 0)
    .map(([motor, duty]) => `M${motor} ${duty}`)
    .join(" · ");
}

export function Cycles({
  device,
  onOpenRaw,
}: {
  device: DeviceApi;
  /** Doc 11: a cycle, and with it its error, jumps to its raw range. */
  onOpenRaw: (focus: RawFocus) => void;
}) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState("");
  const [cycles, setCycles] = useState<Cycle[]>([]);
  const [segments, setSegments] = useState<Segment[]>([]);
  const [haptics, setHaptics] = useState<HapticRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.sessions().then(setSessions).catch((e) => setError(String(e)));
  }, []);

  const load = useCallback(async (sessionId: string) => {
    if (!sessionId) {
      setCycles([]);
      setSegments([]);
      setHaptics([]);
      return;
    }
    try {
      const [c, g, h] = await Promise.all([
        api.cycles(sessionId),
        api.segments(sessionId),
        api.haptics(sessionId),
      ]);
      setCycles(c);
      setSegments(g);
      setHaptics(h);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    load(selected);
  }, [selected, load]);

  const session = sessions.find((s) => s.session_id === selected);
  // A session was scored when it was started against a reference. Reading it
  // from the session, not from the numbers, is what keeps an unscored cycle's
  // all-zero row from being drawn as perfect agreement.
  const isScored = session?.reference_id != null;
  const vocabulary = device.vocabulary;

  const valid = useMemo(() => cycles.filter((c) => c.valid), [cycles]);
  const measured = useMemo(
    () => valid.filter((c) => distanceMeasured(c, vocabulary)),
    [valid, vocabulary],
  );
  const scored = useMemo(
    () => (isScored ? valid.filter((c) => c.confidence > 0) : []),
    [valid, isScored],
  );
  const errorScore = summary(scored.map((c) => c.error_score));
  const confidence = summary(scored.map((c) => c.confidence));
  // Only classifications the engine says may be shown are counted.
  const classCounts = useMemo(() => {
    const counts = new Map<number, number>();
    for (const c of scored) {
      if (c.confidence < CONFIDENCE_FOR_DISPLAY) continue;
      counts.set(c.primary_class, (counts.get(c.primary_class) ?? 0) + 1);
    }
    return [...counts.entries()].sort((a, b) => b[1] - a[1]);
  }, [scored]);
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
                  {sessionTitle(s)} — {s.patient_name}
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
              whose zero-velocity quality reaches{" "}
              {vocabulary?.distance_min_zupt_quality.toFixed(2) ?? "the threshold"} and whose
              stride and speed are within walking; the rest are shown per cycle but not
              counted, rather than corrected (doc 05 §8).
            </p>
          </div>

          {isScored && (
            <div className="panel">
              <h2>Against the reference</h2>
              <div className="readouts">
                <div className="readout">
                  <span className="label">Scored cycles</span>
                  <span className="value num">{scored.length}</span>
                </div>
                <div className="readout">
                  <span className="label">Error score median</span>
                  <span className="value num">
                    {errorScore ? errorScore.median.toFixed(2) : <span className="absent">—</span>}
                  </span>
                </div>
                <div className="readout">
                  <span className="label">Error score MAD</span>
                  <span className="value num">
                    {errorScore ? errorScore.mad.toFixed(2) : <span className="absent">—</span>}
                  </span>
                </div>
                <div className="readout">
                  <span className="label">Confidence median</span>
                  <span className="value num">
                    {confidence ? confidence.median.toFixed(2) : <span className="absent">—</span>}
                  </span>
                </div>
              </div>
              <p className="hint" style={{ marginTop: 10 }}>
                Scored against reference <span className="num">{session?.reference_id}</span>. An
                error score is a weighted distance from that patient's own medians, not a
                measure of health; doc 06 defines no threshold that separates a good cycle
                from a bad one, so none is drawn here.
              </p>
              {classCounts.length > 0 && (
                <table style={{ marginTop: 12 }}>
                  <thead>
                    <tr>
                      <th>Primary class</th>
                      <th>Cycles</th>
                    </tr>
                  </thead>
                  <tbody>
                    {classCounts.map(([index, count]) => (
                      <tr key={index}>
                        <td>{className(vocabulary, index)}</td>
                        <td className="num">{count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              <p className="hint">
                Counted only where confidence reaches {CONFIDENCE_FOR_DISPLAY.toFixed(2)}, the
                level below which doc 06 §7 says the classification should not be shown at all.
              </p>
            </div>
          )}

          {segments.length > 0 && (
            <div className="panel">
              <h2>Segments</h2>
              <p className="hint">
                Limits entered for this session: {session?.max_cycles_per_segment} valid cycles
                or {session?.max_errors_per_segment} errors, whichever came first (doc 12 §5).
              </p>
              <table>
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Started</th>
                    <th>Valid cycles</th>
                    <th>Errors</th>
                    <th>Closed by</th>
                  </tr>
                </thead>
                <tbody>
                  {segments.map((g) => (
                    <tr key={g.segment_index}>
                      <td className="num">{g.segment_index}</td>
                      <td className="num">{g.started_at.slice(11, 19)}</td>
                      <td className="num">{g.valid_cycles}</td>
                      <td className="num">{g.errors}</td>
                      <td>
                        {g.closed_by ? (
                          g.closed_by.replace(/_/g, " ")
                        ) : (
                          <span className="absent">open</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

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
                    {isScored && (
                      <>
                        <th>Segment</th>
                        <th>Score</th>
                        <th>Conf.</th>
                        <th>Class</th>
                        <th title="Motors and duty (of 255) of the cue this cycle produced (DEC-023)">
                          Vibration
                        </th>
                      </>
                    )}
                  </tr>
                </thead>
                <tbody>
                  {cycles.map((c, index) => (
                    <tr
                      key={c.start_frame}
                      style={{ cursor: "pointer", ...(c.valid ? {} : { color: "#868c93" }) }}
                      title="Open this cycle in Raw data"
                      onClick={() =>
                        onOpenRaw({ sessionId: selected, from: c.start_frame, to: c.end_frame })
                      }
                    >
                      <td className="num">{index + 1}</td>
                      <td className="num">
                        {((c.start_us - cycles[0].start_us) / 1e6).toFixed(1)}
                      </td>
                      <td className="num">{c.cycle_time_s.toFixed(2)}</td>
                      <td className="num">{c.stance_ratio.toFixed(2)}</td>
                      <td className="num">{c.cadence_steps_per_min.toFixed(0)}</td>
                      <td className="num">
                        {distanceMeasured(c, vocabulary) ? (
                          c.distance_m.toFixed(2)
                        ) : (
                          <span className="absent" style={{ fontSize: 13 }}>
                            {c.distance_m.toFixed(2)} not measured
                          </span>
                        )}
                      </td>
                      <td className="num">{c.speed_mps.toFixed(2)}</td>
                      <td className="num">{c.zupt_quality.toFixed(2)}</td>
                      <td className="num">{c.peak_dorsiflexion_deg.toFixed(1)}</td>
                      <td className="num">{c.contact_sagittal_deg.toFixed(1)}</td>
                      <td className="num">{c.peak_shank_rate_dps.toFixed(0)}</td>
                      {isScored && (
                        <>
                          <td className="num">{c.segment_index}</td>
                          <td className="num">
                            {c.confidence > 0 ? (
                              c.error_score.toFixed(2)
                            ) : (
                              <span className="absent">not scored</span>
                            )}
                          </td>
                          <td className="num">
                            {c.confidence > 0 ? (
                              c.confidence.toFixed(2)
                            ) : (
                              <span className="absent">—</span>
                            )}
                          </td>
                          <td style={{ fontSize: 13 }}>
                            {c.confidence >= CONFIDENCE_FOR_DISPLAY ? (
                              className(vocabulary, c.primary_class)
                            ) : (
                              <span className="absent">
                                {c.confidence > 0 ? "low confidence" : "—"}
                              </span>
                            )}
                          </td>
                          <td className="num" style={{ fontSize: 13 }}>
                            {cueText(haptics, c.start_frame)}
                          </td>
                        </>
                      )}
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
              {PRIMARY_TRENDS.map((trend) =>
                // The error and haptic trends belong to a scored session; for any
                // other session the panel is absent, not flat at zero.
                trend.scoredOnly && !isScored ? null : (
                  <Trend key={trend.label} valid={valid} cues={haptics} trend={trend} />
                ),
              )}
            </div>
            <h3 style={{ marginTop: 16 }}>Other measurements</h3>
            <div className="trend-grid">
              {OTHER_TRENDS.map((trend) =>
                trend.scoredOnly && !isScored ? null : (
                  <Trend key={trend.label} valid={valid} cues={haptics} trend={trend} />
                ),
              )}
            </div>
          </div>
        </>
      )}
    </>
  );
}
