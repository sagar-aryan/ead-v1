/**
 * EXPORT: the doc 10 package for one session.
 *
 * Everything is derived from the database, never from a live device, so a
 * session recorded last month exports the same bytes today. What the export
 * wrote is reported back as counts rather than as a success message: a package
 * that quietly held fewer rows than the store would be worse than one that
 * failed.
 */
import { useCallback, useEffect, useState } from "react";

import { api, type ExportSummary, type Session } from "../api";

const FILES: { name: string; what: string }[] = [
  { name: "raw.csv", what: "Every stored sample, two rows per frame (doc 10 §2)" },
  { name: "gait.csv", what: "One row per cycle with its score and class (§3)" },
  { name: "events.csv", what: "Device events, cycle bounds, error transitions, faults (§4)" },
  { name: "haptics.csv", what: "Header only: no ERM drivers are fitted (§5, DEC-006)" },
  { name: "metadata.json", what: "Device, calibration, reference, segmentation (§6)" },
  { name: "session.mat", what: "MATLAB Level-5, raw counts kept as integers (§7)" },
];

export function Export() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [selected, setSelected] = useState("");
  const [directory, setDirectory] = useState("");
  const [summary, setSummary] = useState<ExportSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.sessions().then(setSessions).catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    setSummary(null);
    if (!selected) {
      setDirectory("");
      return;
    }
    api.defaultExportDirectory(selected).then(setDirectory).catch(() => setDirectory(""));
  }, [selected]);

  const run = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      setSummary(await api.exportSession(selected, directory || null));
    } catch (e) {
      setError(String(e));
      setSummary(null);
    } finally {
      setBusy(false);
    }
  }, [selected, directory]);

  const session = sessions.find((s) => s.session_id === selected);

  return (
    <>
      <h1>Export</h1>
      <p className="page-hint">
        The doc 10 package: raw samples, cycles, events, haptics, metadata and a
        MATLAB file. Research and engineering data, not a validated clinical record.
      </p>

      <div className="panel">
        <div className="row">
          <div className="field">
            <label htmlFor="export-session">Session</label>
            <select
              id="export-session"
              value={selected}
              onChange={(e) => setSelected(e.target.value)}
            >
              <option value="">Select…</option>
              {sessions.map((s) => (
                <option key={s.session_id} value={s.session_id}>
                  {s.session_id} — {s.patient_name} ({s.kind.replace(/_/g, " ")})
                </option>
              ))}
            </select>
          </div>
          <div className="field" style={{ flex: 1, minWidth: 280 }}>
            <label htmlFor="export-dir">Directory</label>
            <input
              id="export-dir"
              value={directory}
              onChange={(e) => setDirectory(e.target.value)}
            />
          </div>
          <button className="primary" disabled={!selected || busy} onClick={run}>
            {busy ? "Writing…" : "Write package"}
          </button>
        </div>
        {session && (
          <p className="hint">
            {session.frames_stored.toLocaleString()} frames stored
            {session.frames_missing > 0 &&
              `, ${session.frames_missing.toLocaleString()} never arrived`}
            . A session still recording exports what it has so far.
          </p>
        )}
        {error && <p className="error">{error}</p>}
      </div>

      {summary && (
        <div className="panel">
          <h2>Written</h2>
          <div className="readouts">
            <div className="readout">
              <span className="label">Raw rows</span>
              <span className="value num">{summary.raw_rows.toLocaleString()}</span>
            </div>
            <div className="readout">
              <span className="label">Cycle rows</span>
              <span className="value num">{summary.gait_rows.toLocaleString()}</span>
            </div>
            <div className="readout">
              <span className="label">Event rows</span>
              <span className="value num">{summary.event_rows.toLocaleString()}</span>
            </div>
            <div className="readout wide">
              <span className="label">Directory</span>
              <span className="value num" style={{ fontSize: 13 }}>
                {summary.directory}
              </span>
            </div>
          </div>
        </div>
      )}

      <div className="panel">
        <h2>What the package contains</h2>
        <table>
          <thead>
            <tr>
              <th>File</th>
              <th>Contents</th>
              <th>Written</th>
            </tr>
          </thead>
          <tbody>
            {FILES.map((f) => (
              <tr key={f.name}>
                <td className="num">{f.name}</td>
                <td>{f.what}</td>
                <td>
                  {summary ? (
                    summary.files.includes(f.name) ? (
                      "yes"
                    ) : (
                      <span className="absent">no</span>
                    )
                  ) : (
                    <span className="absent">—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="hint">
          Raw values are the chip-frame ADC counts as stored (DEC-007), not physical
          units. The scale factors and both mount maps travel in metadata.json, so
          the conversion is one multiplication away and nothing was rounded on the
          way out.
        </p>
      </div>
    </>
  );
}
