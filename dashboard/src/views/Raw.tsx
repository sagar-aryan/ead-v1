/**
 * RAW: the stored samples for a recorded session.
 *
 * Long recordings are drawn as a min/max envelope per bucket rather than by
 * sampling, so a single-frame impact stays visible; zooming in reaches the exact
 * samples. The backend does the decimation and the conversion to anatomical
 * physical units, using the configuration stored with the session.
 *
 * Two things make this readable as a research view rather than a picture of a
 * signal:
 *
 *  - Every selected signal is drawn at once, one above the other on a shared
 *    time axis with a shared cursor. Gait is read by comparing sensors at the
 *    same instant — a foot impact against what the shank was doing — so showing
 *    one signal at a time would hide the relationship being looked for.
 *  - Navigation is focus plus context. The overview strip always covers the
 *    whole session with the visible window marked on it, so zooming in never
 *    loses the reader's place; dragging on any chart selects a range, and the
 *    pan controls move the window along the recording without changing its
 *    width.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import uPlot from "uplot";

import { api, type RawSignal, type RawWindow, type Session, type SignalGroup } from "../api";
import { MIN_SPAN, clock, nearest, placeWindow } from "../timeline";

const GROUPS: { id: SignalGroup; label: string }[] = [
  { id: "foot_accel", label: "Foot acceleration" },
  { id: "foot_gyro", label: "Foot angular rate" },
  { id: "shank_accel", label: "Shank acceleration" },
  { id: "shank_gyro", label: "Shank angular rate" },
];

const AXIS_COLORS = ["#2a78d6", "#4a3aa7", "#1baf7a"];
const MAX_POINTS = 1400;
const OVERVIEW_POINTS = 700;
const DETAIL_HEIGHT = 160;
const OVERVIEW_HEIGHT = 56;
/** Charts sharing this key share a cursor, so one crosshair reads all signals. */
const SYNC_KEY = "raw";

const frameAtTime = (w: RawWindow, t: number) => w.frame_index[nearest(w.time_s, t)];
const timeAtFrame = (w: RawWindow, frame: number) => w.time_s[nearest(w.frame_index, frame)];

const AXIS_STYLE = {
  stroke: "#868c93",
  grid: { stroke: "#eceeed", width: 1 },
  ticks: { stroke: "#e3e4e2", width: 1 },
  font: '11px "IBM Plex Mono", monospace',
};

function EnvelopeChart({
  data,
  signal,
  height,
  onSelect,
  highlight,
}: {
  /** The window's shared time base. */
  data: RawWindow;
  signal: RawSignal;
  height: number;
  /** A range dragged out on the chart, as frame indices. */
  onSelect: (firstFrame: number, lastFrame: number) => void;
  /** Frame range to mark, for the overview strip. */
  highlight?: [number, number] | null;
}) {
  const host = useRef<HTMLDivElement>(null);
  const plot = useRef<uPlot | null>(null);
  // Held in refs so a new callback or a moved window never rebuilds the plot.
  const select = useRef(onSelect);
  select.current = onSelect;
  const marked = useRef(highlight);
  marked.current = highlight;

  /** Draws the marked window, in CSS pixels over the plotting area. */
  const mark = useCallback(() => {
    const p = plot.current;
    const range = marked.current;
    if (!p || !range || data.points === 0) return;
    const left = p.valToPos(timeAtFrame(data, range[0]), "x");
    const right = p.valToPos(timeAtFrame(data, range[1]), "x");
    p.setSelect(
      { left, top: 0, width: Math.max(2, right - left), height: p.over.clientHeight },
      false,
    );
  }, [data]);

  useEffect(() => {
    if (!host.current) return;
    const series: uPlot.Series[] = [{ label: "t (s)" }];
    const values: (number[] | Float64Array)[] = [data.time_s];
    const bands: uPlot.Band[] = [];
    signal.axes.forEach((axis, index) => {
      // Two series per axis (max then min) joined as a band: the shaded area is
      // the full range of samples inside each bucket.
      values.push(axis.max, axis.min);
      const color = AXIS_COLORS[index];
      series.push({ label: `${axis.axis} max`, stroke: color, width: 1.5, points: { show: false } });
      series.push({ label: `${axis.axis} min`, stroke: color, width: 1.5, points: { show: false } });
      bands.push({ series: [series.length - 2, series.length - 1], fill: `${color}22` });
    });

    const chart = new uPlot(
      {
        width: host.current.clientWidth || 800,
        height,
        series,
        bands,
        legend: { show: false },
        cursor: {
          // Dragging picks a range to look at. setScale stays off because the
          // backend re-queries that range at full resolution rather than us
          // stretching the points already drawn.
          drag: { x: true, y: false, setScale: false },
          // One crosshair across every stacked signal, matched by time value.
          sync: { key: SYNC_KEY, setSeries: false, scales: ["x", null] },
        },
        hooks: {
          setSelect: [
            (p: uPlot) => {
              if (p.select.width <= 0 || data.points === 0) return;
              const from = p.posToVal(p.select.left, "x");
              const to = p.posToVal(p.select.left + p.select.width, "x");
              select.current(frameAtTime(data, from), frameAtTime(data, to));
              // Clearing lets the overview repaint the window it was given; the
              // width guard above stops this from re-entering.
              p.setSelect({ left: 0, top: 0, width: 0, height: 0 }, false);
              mark();
            },
          ],
        },
        // The x axis is elapsed seconds, not wall-clock time: without this
        // uPlot formats it as a date from the epoch.
        scales: { x: { time: false } },
        // Every chart carries the same y-axis width so the stacked signals and
        // the overview share one vertical grid the eye can read down.
        axes: [AXIS_STYLE, { ...AXIS_STYLE, size: 54 }],
      },
      values as unknown as uPlot.AlignedData,
      host.current,
    );
    plot.current = chart;
    mark();

    const resize = () => {
      if (!host.current) return;
      chart.setSize({ width: host.current.clientWidth, height });
      mark();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host.current);
    return () => {
      observer.disconnect();
      chart.destroy();
      plot.current = null;
    };
  }, [data, signal, height, mark]);

  useEffect(mark, [mark, highlight]);

  return <div ref={host} style={{ width: "100%", height }} />;
}

export function Raw() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState("");
  const [groups, setGroups] = useState<SignalGroup[]>(GROUPS.map((g) => g.id));
  const [detail, setDetail] = useState<RawWindow | null>(null);
  const [overview, setOverview] = useState<RawWindow | null>(null);
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

  const first = session?.first_frame_index ?? null;
  const last = session?.last_frame_index ?? null;
  const empty = first === null || last === null;
  // Identity is what the load effects depend on, so compare by content.
  const groupKey = groups.join(",");
  /** The overview is context for all of them, so it draws the topmost signal. */
  const contextGroup = groups[0] ?? null;

  useEffect(() => setRange(null), [selected]);

  // The overview covers the whole session, so it only reloads when the session
  // or the context signal changes: panning and zooming leave it alone.
  useEffect(() => {
    if (!selected || empty || !contextGroup) {
      setOverview(null);
      return;
    }
    let live = true;
    api
      .rawWindow(selected, [contextGroup], first, last, OVERVIEW_POINTS)
      .then((w) => live && setOverview(w))
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [selected, contextGroup, first, last, empty]);

  const load = useCallback(
    async (from: number, to: number) => {
      setLoading(true);
      try {
        // One request for every signal: they read the same rows, so the store
        // aggregates them in a single pass on one shared time base.
        setDetail(await api.rawWindow(selected, groups, from, to, MAX_POINTS));
        setError(null);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    },
    // groups is compared by content (groupKey), so an equal list never reloads.
    [selected, groupKey],
  );

  useEffect(() => {
    if (!selected || empty || groups.length === 0) {
      setDetail(null);
      return;
    }
    const [from, to] = range ?? [first, last];
    load(from, to);
  }, [selected, empty, first, last, range, load, groupKey]);

  const place = useCallback(
    (from: number, span: number) => {
      if (empty) return;
      setRange(placeWindow({ first, last }, from, span));
    },
    [empty, first, last],
  );

  const view: [number, number] | null = empty ? null : (range ?? [first, last]);
  const span = view ? view[1] - view[0] + 1 : 0;

  const pan = useCallback(
    (direction: number) => {
      if (!view) return;
      place(view[0] + direction * Math.round(span / 2), span);
    },
    [view, span, place],
  );

  const zoom = useCallback(
    (factor: number) => {
      if (!view) return;
      const centre = (view[0] + view[1]) / 2;
      const width = span * factor;
      place(centre - width / 2, width);
    },
    [view, span, place],
  );

  // Arrow keys pan and +/- zoom, which is how a reader scrubs through a
  // recording without going back to the mouse for every step.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const tag = (event.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
      if (event.key === "ArrowLeft") pan(-1);
      else if (event.key === "ArrowRight") pan(1);
      else if (event.key === "-") zoom(2);
      else if (event.key === "+" || event.key === "=") zoom(0.5);
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pan, zoom]);

  const toggle = (id: SignalGroup) =>
    setGroups((current) =>
      current.includes(id)
        ? current.filter((g) => g !== id)
        : GROUPS.map((g) => g.id).filter((g) => g === id || current.includes(g)),
    );

  const atStart = empty || !view || view[0] <= first;
  const atEnd = empty || !view || view[1] >= last;
  const shown = detail;

  return (
    <>
      <h1>Raw data</h1>
      <p className="page-hint">
        Every stored sample, exactly as the device sent it. A long range is drawn
        as the full span of values inside each bucket, so a single-frame impact is
        never averaged away. The selected signals share one time axis and one
        cursor, so a moment can be read across both sensors at once.
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
            <label>Move</label>
            <div className="button-group">
              <button onClick={() => pan(-1)} disabled={atStart} title="Earlier (left arrow)">
                ◀
              </button>
              <button onClick={() => pan(1)} disabled={atEnd} title="Later (right arrow)">
                ▶
              </button>
            </div>
          </div>
          <div className="field">
            <label>Zoom</label>
            <div className="button-group">
              <button onClick={() => zoom(0.5)} disabled={!view || span <= MIN_SPAN} title="In (+)">
                In
              </button>
              <button onClick={() => zoom(2)} disabled={!range} title="Out (−)">
                Out
              </button>
              <button onClick={() => setRange(null)} disabled={!range}>
                Whole session
              </button>
            </div>
          </div>
        </div>
        <div className="row" style={{ marginTop: 12 }}>
          <div className="field">
            <label>Signals</label>
            <div className="toggle-group">
              {GROUPS.map((g) => (
                <label key={g.id} className="toggle">
                  <input
                    type="checkbox"
                    checked={groups.includes(g.id)}
                    onChange={() => toggle(g.id)}
                  />
                  <span className={`sensor ${g.id.startsWith("foot") ? "foot" : "shank"}`}>
                    <span className="name">{g.label}</span>
                  </span>
                </label>
              ))}
            </div>
          </div>
        </div>
        {error && <p className="error">{error}</p>}
      </div>

      {!selected && <p className="empty">Select a recorded session.</p>}
      {selected && empty && <p className="empty">This session stored no frames.</p>}
      {selected && !empty && groups.length === 0 && <p className="empty">Select a signal.</p>}

      {overview && shown && (
        <div className="panel">
          <div className="chart-title">
            <span className="name">Whole session — {overview.signals[0].label}</span>
            <span className="legend num">
              {overview.points > 0 &&
                `${clock(overview.time_s[0])} – ${clock(overview.time_s[overview.points - 1])}`}
            </span>
          </div>
          <EnvelopeChart
            data={overview}
            signal={overview.signals[0]}
            height={OVERVIEW_HEIGHT}
            onSelect={(from, to) => place(from, to - from + 1)}
            highlight={view}
          />
          <p className="hint">
            Drag on any chart to select a range. Arrow keys move the window, + and
            − change its width.
          </p>
        </div>
      )}

      {shown && (
        <div className="panel">
          {shown.signals.map((signal) => (
            <div key={signal.group} className="stacked-signal">
              <div className="chart-title">
                <span className="name">
                  <span className={`sensor ${signal.sensor}`}>
                    <span className="name">
                      {signal.label} ({signal.unit})
                    </span>
                  </span>
                </span>
                <span className="legend">
                  {signal.axes.map((axis, axisIndex) => (
                    <span key={axis.axis} style={{ color: AXIS_COLORS[axisIndex] }}>
                      {axis.axis}
                    </span>
                  ))}
                </span>
              </div>
              <EnvelopeChart
                data={shown}
                signal={signal}
                height={DETAIL_HEIGHT}
                onSelect={(from, to) => place(from, to - from + 1)}
              />
            </div>
          ))}
          <div className="readouts" style={{ marginTop: 12 }}>
            <div className="readout wide">
              <span className="label">Window</span>
              <span className="value num">
                {shown.points > 0
                  ? `${clock(shown.time_s[0])} – ${clock(shown.time_s[shown.points - 1])}`
                  : "—"}
              </span>
            </div>
            <div className="readout">
              <span className="label">Frames shown</span>
              <span className="value num">{span.toLocaleString()}</span>
            </div>
            <div className="readout">
              <span className="label">Frames per point</span>
              <span className="value num">{shown.bucket}</span>
            </div>
            <div className="readout">
              <span className="label">Points drawn</span>
              <span className="value num">{shown.points.toLocaleString()}</span>
            </div>
            <div className="readout">
              <span className="label">Query</span>
              <span className="value num">
                {shown.query_ms}
                <span className="unit">ms</span>
              </span>
            </div>
            <div className="readout">
              <span className="label">Flagged buckets</span>
              <span className="value num">{shown.status.filter((s) => s !== 0).length}</span>
            </div>
          </div>
          {!shown.anatomical && (
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
