/**
 * PATIENTS and SESSIONS: create a patient, record a raw dataset, and review
 * what was stored. Recording keeps every 100 Hz frame; the count and any gap
 * are shown so a dataset is never silently incomplete.
 */
import { useCallback, useEffect, useState } from "react";

import { api, type Patient, type Session } from "../api";
import type { DeviceApi } from "../useDevice";

function useSessions() {
  const [patients, setPatients] = useState<Patient[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [recording, setRecording] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [p, s, r] = await Promise.all([
        api.patients(),
        api.sessions(),
        api.recordingSession(),
      ]);
      setPatients(p);
      setSessions(s);
      setRecording(r);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 1000);
    return () => clearInterval(timer);
  }, [refresh]);

  return { patients, sessions, recording, error, setError, refresh };
}

function duration(session: Session): string {
  if (!session.stopped_at) return "recording";
  const seconds =
    (Date.parse(session.stopped_at) - Date.parse(session.started_at)) / 1000;
  if (!Number.isFinite(seconds)) return "—";
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes}m ${Math.round(seconds % 60)}s` : `${seconds.toFixed(1)}s`;
}

export function Sessions({ device }: { device: DeviceApi }) {
  const { patients, sessions, recording, error, setError, refresh } = useSessions();
  const [patientId, setPatientId] = useState("");
  const [name, setName] = useState("");
  const [selected, setSelected] = useState("");

  const connected = device.snapshot?.link_state === "connected";

  const create = async () => {
    try {
      await api.createPatient(patientId, name);
      setPatientId("");
      setName("");
      setError(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const start = async () => {
    try {
      await api.startRecording(selected);
      setError(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const stop = async () => {
    try {
      await api.stopRecording();
      setError(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <h1>Sessions</h1>
      <p className="page-hint">
        A recording stores every raw sample exactly as the device sent it, together
        with the firmware version and configuration that produced it.
      </p>

      <div className="panel">
        <h2>Record</h2>
        {patients.length === 0 ? (
          <p className="empty">Create a patient first.</p>
        ) : recording ? (
          <div className="row">
            <p className="hint" style={{ margin: 0, flex: 1 }}>
              Recording <span className="num">{recording}</span>
              {device.snapshot?.status && (
                <>
                  {" — "}
                  <span className="num">
                    {device.snapshot.frames_received.toLocaleString()}
                  </span>{" "}
                  frames received on this link
                </>
              )}
            </p>
            <button className="danger" onClick={stop}>
              Stop recording
            </button>
          </div>
        ) : (
          <div className="row">
            <div className="field">
              <label htmlFor="patient">Patient</label>
              <select id="patient" value={selected} onChange={(e) => setSelected(e.target.value)}>
                <option value="">Select…</option>
                {patients.map((p) => (
                  <option key={p.patient_id} value={p.patient_id}>
                    {p.name} ({p.patient_id})
                  </option>
                ))}
              </select>
            </div>
            <button className="primary" disabled={!connected || selected === ""} onClick={start}>
              Start recording
            </button>
            {!connected && <p className="hint" style={{ margin: 0 }}>Connect the device first.</p>}
          </div>
        )}
        {error && <p className="error">{error}</p>}
      </div>

      <div className="panel">
        <h2>Patients</h2>
        <div className="row" style={{ marginBottom: 14 }}>
          <div className="field">
            <label htmlFor="pid">Study ID</label>
            <input id="pid" value={patientId} onChange={(e) => setPatientId(e.target.value)} />
          </div>
          <div className="field">
            <label htmlFor="pname">Name</label>
            <input id="pname" value={name} onChange={(e) => setName(e.target.value)} />
          </div>
          <button onClick={create} disabled={patientId.trim() === "" || name.trim() === ""}>
            Add patient
          </button>
        </div>
        {patients.length === 0 ? (
          <p className="empty">No patients yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Study ID</th>
                <th>Name</th>
                <th>Sessions</th>
                <th>Created</th>
              </tr>
            </thead>
            <tbody>
              {patients.map((p) => (
                <tr key={p.patient_id}>
                  <td className="num">{p.patient_id}</td>
                  <td>{p.name}</td>
                  <td className="num">{p.session_count}</td>
                  <td className="num">{p.created_at.slice(0, 19).replace("T", " ")}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="panel">
        <h2>Recorded sessions</h2>
        {sessions.length === 0 ? (
          <p className="empty">No sessions yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Session</th>
                <th>Patient</th>
                <th>Started</th>
                <th>Duration</th>
                <th>Frames</th>
                <th>Missing</th>
                <th>Firmware</th>
              </tr>
            </thead>
            <tbody>
              {sessions.map((s) => (
                <tr key={s.session_id}>
                  <td className="num">{s.session_id}</td>
                  <td>{s.patient_name}</td>
                  <td className="num">{s.started_at.slice(0, 19).replace("T", " ")}</td>
                  <td className="num">{duration(s)}</td>
                  <td className="num">{s.frames_stored.toLocaleString()}</td>
                  <td className="num">
                    {s.frames_missing > 0 ? (
                      <span style={{ color: "var(--critical)" }}>{s.frames_missing}</span>
                    ) : (
                      0
                    )}
                  </td>
                  <td className="num" style={{ fontSize: 12 }}>
                    {s.firmware ?? <span className="absent">—</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </>
  );
}
