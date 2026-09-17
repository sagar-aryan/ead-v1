/**
 * EVENTS: what the detector decided, and when.
 *
 * One lane per event kind over the session's timeline, so a missed contact or a
 * zero-velocity window that opened during swing is visible as a gap or an
 * overlap rather than as a number that looks slightly wrong three views away.
 *
 * Times are elapsed device time from the session's first event. The device's
 * sample period is not exactly 10 ms (TEST-018), so every time here comes from
 * the device's own timestamps and never from a frame index times 0.01.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { api, type Cycle, type GaitEvent, type Session } from "../api";
import { zuptSpans, type Span } from "../events";
import { clock } from "../timeline";

const LANES = [
  { kind: "initial_contact", label: "Initial contact" },
  { kind: "toe_off", label: "Toe off" },
  { kind: "foot_flat", label: "Foot flat" },
  { kind: "zupt", label: "Zero velocity" },
  { kind: "cycle", label: "Cycles" },
] as const;

const LANE_HEIGHT = 34;
const AXIS_HEIGHT = 26;
const LABEL_WIDTH = 132;
const INK = "#15181c";
const MUTED = "#868c93";
const GRID = "#eceeed";

export function Events() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState("");
  const [events, setEvents] = useState<GaitEvent[]>([]);
  const [cycles, setCycles] = useState<Cycle[]>([]);
  const [error, setError] = useState<string | null>(null);
  /** Device-time window in µs; null is the whole session. */
  const [window, setWindow] = useState<Span | null>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const drag = useRef<{ from: number; to: number } | null>(null);

  useEffect(() => {
    api.sessions().then(setSessions).catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    setWindow(null);
    if (!selected) {
      setEvents([]);
      setCycles([]);
      return;
    }
    Promise.all([api.events(selected), api.cycles(selected)])
      .then(([e, c]) => {
        setEvents(e);
        setCycles(c);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  }, [selected]);

  const bounds = useMemo((): Span | null => {
    const times = [
      ...events.map((e) => e.timestamp_us),
      ...cycles.map((c) => c.start_us),
      ...cycles.map((c) => c.start_us + c.cycle_time_s * 1e6),
    ];
    if (times.length === 0) return null;
    return { from: Math.min(...times), to: Math.max(...times) };
  }, [events, cycles]);

  const view = window ?? bounds;

  const draw = useCallback(() => {
    const element = canvas.current;
    if (!element || !view || !bounds) return;
    const width = element.clientWidth;
    const height = LANES.length * LANE_HEIGHT + AXIS_HEIGHT;
    const ratio = globalThis.devicePixelRatio || 1;
    element.width = Math.round(width * ratio);
    element.height = Math.round(height * ratio);
    element.style.height = `${height}px`;
    const g = element.getContext("2d");
    if (!g) return;
    g.setTransform(ratio, 0, 0, ratio, 0, 0);
    g.clearRect(0, 0, width, height);
    g.font = '11px "IBM Plex Mono", monospace';

    const span = Math.max(1, view.to - view.from);
    const plotWidth = width - LABEL_WIDTH;
    const x = (us: number) => LABEL_WIDTH + ((us - view.from) / span) * plotWidth;

    // Lane frames and labels.
    LANES.forEach((lane, i) => {
      const top = i * LANE_HEIGHT;
      g.strokeStyle = GRID;
      g.lineWidth = 1;
      g.beginPath();
      g.moveTo(0, top + 0.5);
      g.lineTo(width, top + 0.5);
      g.stroke();
      g.fillStyle = MUTED;
      g.fillText(lane.label, 6, top + LANE_HEIGHT / 2 + 4);
    });

    // Time axis: five labelled ticks across the window.
    const axisTop = LANES.length * LANE_HEIGHT;
    g.strokeStyle = GRID;
    g.beginPath();
    g.moveTo(0, axisTop + 0.5);
    g.lineTo(width, axisTop + 0.5);
    g.stroke();
    g.fillStyle = MUTED;
    for (let i = 0; i <= 5; i += 1) {
      const us = view.from + (span * i) / 5;
      const px = x(us);
      g.strokeStyle = GRID;
      g.beginPath();
      g.moveTo(px, 0);
      g.lineTo(px, axisTop);
      g.stroke();
      const label = clock((us - bounds.from) / 1e6);
      g.fillText(label, Math.min(px + 4, width - 42), axisTop + 16);
    }

    const laneTop = (kind: string) =>
      LANES.findIndex((l) => l.kind === kind) * LANE_HEIGHT;

    // Cycles first, so the event ticks sit on top of their bars.
    const cycleTop = laneTop("cycle");
    for (const c of cycles) {
      const from = x(c.start_us);
      const to = x(c.start_us + c.cycle_time_s * 1e6);
      if (to < LABEL_WIDTH || from > width) continue;
      g.fillStyle = c.valid ? "#dfe3e6" : "#f2f0ec";
      g.fillRect(from, cycleTop + 8, Math.max(1, to - from - 1), LANE_HEIGHT - 16);
      // A cycle the engine classified is outlined, so an error lines up with
      // the events that produced it without inventing a colour scale.
      if (c.valid && c.primary_class !== 0 && c.confidence >= 0.5) {
        g.strokeStyle = INK;
        g.lineWidth = 1.5;
        g.strokeRect(from + 0.5, cycleTop + 8.5, Math.max(1, to - from - 2), LANE_HEIGHT - 17);
      }
    }

    // Zero-velocity windows as bands.
    const zuptTop = laneTop("zupt");
    g.fillStyle = "#dfe3e6";
    for (const s of zuptSpans(events, bounds.to)) {
      const from = x(s.from);
      const to = x(s.to);
      if (to < LABEL_WIDTH || from > width) continue;
      g.fillRect(from, zuptTop + 8, Math.max(1, to - from), LANE_HEIGHT - 16);
    }

    // Event ticks.
    g.strokeStyle = INK;
    g.lineWidth = 1.5;
    for (const e of events) {
      const top = laneTop(e.kind);
      if (top < 0) continue;
      const px = x(e.timestamp_us);
      if (px < LABEL_WIDTH || px > width) continue;
      g.beginPath();
      g.moveTo(px, top + 9);
      g.lineTo(px, top + LANE_HEIGHT - 9);
      g.stroke();
    }

    // Left gutter last, so nothing draws under the labels.
    g.clearRect(0, 0, LABEL_WIDTH - 6, height);
    g.fillStyle = MUTED;
    LANES.forEach((lane, i) => {
      g.fillText(lane.label, 6, i * LANE_HEIGHT + LANE_HEIGHT / 2 + 4);
    });
  }, [view, bounds, events, cycles]);

  useEffect(() => {
    draw();
    const element = canvas.current;
    if (!element) return;
    const observer = new ResizeObserver(draw);
    observer.observe(element);
    return () => observer.disconnect();
  }, [draw]);

  const timeAt = (clientX: number): number | null => {
    const element = canvas.current;
    if (!element || !view) return null;
    const rect = element.getBoundingClientRect();
    const plotWidth = rect.width - LABEL_WIDTH;
    const fraction = (clientX - rect.left - LABEL_WIDTH) / plotWidth;
    return view.from + Math.min(Math.max(fraction, 0), 1) * (view.to - view.from);
  };

  const counts = useMemo(() => {
    const map = new Map<string, number>();
    for (const e of events) map.set(e.kind, (map.get(e.kind) ?? 0) + 1);
    return map;
  }, [events]);

  return (
    <>
      <h1>Events</h1>
      <p className="page-hint">
        Every event the detector emitted, in device time. Drag across the lanes to
        zoom; the zero-velocity lane is a band, so a window that opens during swing
        shows up against the cycle it belongs to.
      </p>

      <div className="panel">
        <div className="row">
          <div className="field">
            <label htmlFor="event-session">Session</label>
            <select
              id="event-session"
              value={selected}
              onChange={(e) => setSelected(e.target.value)}
            >
              <option value="">Select…</option>
              {sessions.map((s) => (
                <option key={s.session_id} value={s.session_id}>
                  {s.session_id} — {s.patient_name}
                </option>
              ))}
            </select>
          </div>
          {window && <button onClick={() => setWindow(null)}>Whole session</button>}
        </div>
        {error && <p className="error">{error}</p>}
      </div>

      {!selected && <p className="empty">Select a recorded session.</p>}
      {selected && !bounds && (
        <p className="empty">
          No events in this session. The device only emits them once it has been
          calibrated and the patient has walked.
        </p>
      )}

      {bounds && view && (
        <>
          <div className="panel">
            <div className="readouts">
              {LANES.filter((l) => l.kind !== "zupt" && l.kind !== "cycle").map((lane) => (
                <div className="readout" key={lane.kind}>
                  <span className="label">{lane.label}</span>
                  <span className="value num">{counts.get(lane.kind) ?? 0}</span>
                </div>
              ))}
              <div className="readout">
                <span className="label">Zero-velocity windows</span>
                <span className="value num">{zuptSpans(events, bounds.to).length}</span>
              </div>
              <div className="readout">
                <span className="label">Cycles</span>
                <span className="value num">{cycles.length}</span>
              </div>
            </div>
            <p className="hint" style={{ marginTop: 10 }}>
              Showing {clock((view.from - bounds.from) / 1e6)} to{" "}
              {clock((view.to - bounds.from) / 1e6)} of{" "}
              {clock((bounds.to - bounds.from) / 1e6)}.
            </p>
          </div>

          <div className="panel">
            <canvas
              ref={canvas}
              style={{ width: "100%", cursor: "col-resize", display: "block" }}
              onPointerDown={(e) => {
                const at = timeAt(e.clientX);
                if (at === null) return;
                drag.current = { from: at, to: at };
                e.currentTarget.setPointerCapture(e.pointerId);
              }}
              onPointerMove={(e) => {
                if (!drag.current) return;
                const at = timeAt(e.clientX);
                if (at !== null) drag.current.to = at;
              }}
              onPointerUp={() => {
                const selection = drag.current;
                drag.current = null;
                if (!selection) return;
                const from = Math.min(selection.from, selection.to);
                const to = Math.max(selection.from, selection.to);
                // A click rather than a drag: nothing selected, nothing changes.
                if (to - from < 50_000) return;
                setWindow({ from, to });
              }}
            />
            <p className="hint">
              An outlined cycle is one the error engine classified with confidence at or
              above 0.50. Cycles rejected by the temporal guards are drawn paler rather
              than hidden.
            </p>
          </div>
        </>
      )}
    </>
  );
}
