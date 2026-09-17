/**
 * RAW: the stored samples for a recorded session.
 *
 * Long recordings are drawn as a min/max envelope per bucket rather than by
 * sampling, so a single-frame impact stays visible; zooming in reaches the exact
 * samples. The backend does the decimation and the conversion to anatomical
 * physical units, using the configuration stored with the session.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import uPlot from "uplot";

import { api, type RawWindow, type Session, type SignalGroup } from "../api";

const GROUPS: { id: SignalGroup; label: string }[] = [
  { id: "foot_accel", label: "Foot acceleration" },
  { id: "foot_gyro", label: "Foot angular rate" },
  { id: "shank_accel", label: "Shank acceleration" },
  { id: "shank_gyro", label: "Shank angular rate" },
];

const AXIS_COLORS = ["#2a78d6", "#4a3aa7", "#1baf7a"];
const MAX_POINTS = 1400;

function EnvelopeChart({ window: data }: { window: RawWindow }) {
  const host = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!host.current) return;
    const series: uPlot.Series[] = [{ label: "t (s)" }];
    const values: (number[] | Float64Array)[] = [data.time_s];
    const bands: uPlot.Band[] = [];
    data.axes.forEach((axis, index) => {
      // Two series per axis (max then min) joined as a band: the shaded area is
      // the full range of samples inside each bucket.
      values.push(axis.max, axis.min);
      const color = AXIS_COLORS[index];
      series.push({ label: `${axis.axis} max`, stroke: color, width: 1.5, points: { show: false } });
      series.push({ label: `${axis.axis} min`, stroke: color, width: 1.5, points: { show: false } });
      bands.push({ series: [series.length - 2, series.length - 1], fill: `${color}22` });
    });

    const plot = new uPlot(
      {
        width: host.current.clientWidth || 800,
        height: 200,
        series,
        bands,
        legend: { show: false },
        cursor: { drag: { x: false, y: false } },
        // The x axis is elapsed seconds, not wall-clock time: without this
        // uPlot formats it as a date from the epoch.
        scales: { x: { time: false } },
        axes: [
          {
            stroke: "#868c93",
            grid: { stroke: "#eceeed", width: 1 },
            ticks: { stroke: "#e3e4e2", width: 1 },
            font: '11px "IBM Plex Mono", monospace',
          },
          {
            stroke: "#868c93",
            grid: { stroke: "#eceeed", width: 1 },
            ticks: { stroke: "#e3e4e2", width: 1 },
            font: '11px "IBM Plex Mono", monospace',
            size: 54,
          },
        ],
      },
      values as unknown as uPlot.AlignedData,
      host.current,
    );

    const resize = () => {
      if (host.current) plot.setSize({ width: host.current.clientWidth, height: 200 });
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host.current);
    return () => {
      observer.disconnect();
      plot.destroy();
    };
  }, [data]);

  return <div ref={host} style={{ width: "100%", height: 200 }} />;
}

export function Raw() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState("");
  const [group, setGroup] = useState<SignalGroup>("foot_accel");
  const [data, setData] = useState<RawWindow | null>(null);
  const [range, setRange] = useState<[number, number] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    api.sessions().then(setSessions).catch((e) => setError(String(e)));
  }, []);

  const session = useMemo(
    () => sessions.find((s) => s.session_id === selected) ?? null,
    [sessions, selected],
  );

  useEffect(() => setRange(null), [selected]);

  const load = useCallback(
    async (first: number, last: number) => {
      setLoading(true);
      try {
        setData(await api.rawWindow(selected, group, first, last, MAX_POINTS));
        setError(null);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    },
    [selected, group],
  );

  useEffect(() => {
    if (!session || session.first_frame_index === null || session.last_frame_index === null) {
      setData(null);
      return;
    }
    const [first, last] = range ?? [session.first_frame_index, session.last_frame_index];
    load(first, last);
  }, [session, range, load]);

  const zoom = () => {
    if (!data || !session) return;
    const span = data.last_frame - data.first_frame + 1;
    const centre = (data.first_frame + data.last_frame) / 2;
    const half = Math.max(50, Math.round((span * 0.4) / 2));
    setRange([
      Math.max(session.first_frame_index ?? 0, Math.round(centre - half)),
      Math.min(session.last_frame_index ?? 0, Math.round(centre + half)),
    ]);
  };

  return (
    <>
      <h1>Raw data</h1>
      <p className="page-hint">
        Every stored sample, exactly as the device sent it. A long range is drawn
        as the full span of values inside each bucket, so a single-frame impact is
        never averaged away.
      </p>

      <div className="panel">
        <div className="row">
          <div className="field">
            <label htmlFor="session">Session</label>
            <select id="session" value={selected} onChange={(e) => setSelected(e.target.value)}>
              <option value="">Select…</option>
              {sessions.map((s) => (
                <option key={s.session_id} value={s.session_id}>
                  {s.session_id} — {s.patient_name} ({s.frames_stored.toLocaleString()} frames)
                </option>
              ))}
            </select>
          </div>
          <div className="field">
            <label htmlFor="signal">Signal</label>
            <select
              id="signal"
              value={group}
              onChange={(e) => setGroup(e.target.value as SignalGroup)}
            >
              {GROUPS.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.label}
                </option>
              ))}
            </select>
          </div>
          <button onClick={zoom} disabled={!data}>
            Zoom in
          </button>
          <button onClick={() => setRange(null)} disabled={!range}>
            Whole session
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </div>

      {!selected && <p className="empty">Select a recorded session.</p>}

      {data && (
        <div className="panel">
          <div className="chart-title">
            <span className="name">
              <span className={`sensor ${data.sensor}`}>
                <span className="name">
                  {data.label} ({data.unit})
                </span>
              </span>
            </span>
            <span className="legend">
              {data.axes.map((axis, index) => (
                <span key={axis.axis} style={{ color: AXIS_COLORS[index] }}>
                  {axis.axis}
                </span>
              ))}
            </span>
          </div>
          <EnvelopeChart window={data} />
          <div className="readouts" style={{ marginTop: 12 }}>
            <div className="readout">
              <span className="label">Frames shown</span>
              <span className="value num">
                {(data.last_frame - data.first_frame + 1).toLocaleString()}
              </span>
            </div>
            <div className="readout">
              <span className="label">Frames per point</span>
              <span className="value num">{data.bucket}</span>
            </div>
            <div className="readout">
              <span className="label">Points drawn</span>
              <span className="value num">{data.points.toLocaleString()}</span>
            </div>
            <div className="readout">
              <span className="label">Query</span>
              <span className="value num">
                {data.query_ms}
                <span className="unit">ms</span>
              </span>
            </div>
            <div className="readout">
              <span className="label">Flagged buckets</span>
              <span className="value num">{data.status.filter((s) => s !== 0).length}</span>
            </div>
          </div>
          {!data.anatomical && (
            <p className="hint" style={{ marginTop: 10 }}>
              This session stored no device configuration, so values are raw sensor
              counts in the sensor's own axes rather than anatomical units.
            </p>
          )}
          {loading && <p className="hint">Loading…</p>}
        </div>
      )}
    </>
  );
}
