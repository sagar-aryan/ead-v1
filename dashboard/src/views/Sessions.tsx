/**
 * PATIENTS and SESSIONS: create a patient, record a raw dataset, and review
 * what was stored. Recording keeps every 100 Hz frame; the count and any gap
 * are shown so a dataset is never silently incomplete.
 */
import { useCallback, useEffect, useState } from "react";

import { api, type Patient, type Session, type StoredReference } from "../api";
import type { DeviceApi } from "../useDevice";

/**
 * Doc 12 §4: an evaluation may start only with a patient, a reference profile,
 * both segment limits and healthy sensors. The list of what is missing comes
 * from the backend, so the button and the command agree about the gate rather
 * than each deciding for itself.
 */
function Evaluation({
  patients,
  running,
  refresh,
}: {
  patients: Patient[];
  running: Session | null;
  refresh: () => void;
}) {
  const [patientId, setPatientId] = useState("");
  const [references, setReferences] = useState<StoredReference[]>([]);
  const [referenceId, setReferenceId] = useState("");
  // Strings, because doc 12 §5 forbids inventing a default: an empty box must
  // stay empty rather than fall back to a number nobody entered.
  const [maxCycles, setMaxCycles] = useState("");
  const [maxErrors, setMaxErrors] = useState("");
  const [blockers, setBlockers] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const limits =
    /^\d+$/.test(maxCycles) && /^\d+$/.test(maxErrors)
      ? { max_cycles: Number(maxCycles), max_errors: Number(maxErrors) }
      : null;

  useEffect(() => {
    setReferenceId("");
    if (!patientId) {
      setReferences([]);
      return;
    }
    api.references(patientId).then(setReferences).catch((e) => setError(String(e)));
  }, [patientId]);

  useEffect(() => {
    if (running) return;
    const poll = () =>
      api
        .sessionBlockers(patientId, referenceId, limits, "evaluation")
        .then(setBlockers)
        .catch(() => undefined);
    poll();
    const timer = setInterval(poll, 1000);
    return () => clearInterval(timer);
    // `limits` is rebuilt each render; the two fields it derives from are the
    // real dependencies.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [patientId, referenceId, maxCycles, maxErrors, running]);

  const start = async () => {
    try {
      await api.startScoredSession(patientId, referenceId, false, limits);
      setError(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  const stop = async () => {
    try {
      await api.stopScoredSession();
      setError(null);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };

  if (running) {
    return (
      <div className="panel">
        <h2>Evaluation</h2>
        <div className="row">
          <p className="hint" style={{ margin: 0, flex: 1 }}>
            Evaluating <span className="num">{running.session_id}</span> against reference{" "}
            <span className="num">{running.reference_id}</span>. Segments close at{" "}
            <span className="num">{running.max_cycles_per_segment}</span> valid cycles or{" "}
            <span className="num">{running.max_errors_per_segment}</span> errors. Scores
            appear in Cycles.
          </p>
          <button className="danger" onClick={stop}>
            Stop evaluation
          </button>
        </div>
        {error && <p className="error">{error}</p>}
      </div>
    );
  }

  return (
    <div className="panel">
      <h2>Evaluation</h2>
      <p className="hint">
        Scores each cycle against the patient's locked reference and splits the
        session into segments. Both limits are required: doc 12 §5 says no default
        may be invented for them.
      </p>
      <div className="row">
        <div className="field">
          <label htmlFor="eval-patient">Patient</label>
          <select
            id="eval-patient"
            value={patientId}
            onChange={(e) => setPatientId(e.target.value)}
          >
            <option value="">Select…</option>
            {patients.map((p) => (
              <option key={p.patient_id} value={p.patient_id}>
                {p.name} ({p.patient_id})
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label htmlFor="eval-reference">Reference</label>
          <select
            id="eval-reference"
            value={referenceId}
            onChange={(e) => setReferenceId(e.target.value)}
          >
            <option value="">Select…</option>
            {references.map((r) => (
              <option key={r.reference_id} value={r.reference_id}>
                v{r.version} — {r.cycles} cycles{r.locked ? ", locked" : ""}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label htmlFor="eval-cycles">Valid cycles per segment</label>
          <input
            id="eval-cycles"
            inputMode="numeric"
            value={maxCycles}
            onChange={(e) => setMaxCycles(e.target.value)}
          />
        </div>
        <div className="field">
          <label htmlFor="eval-errors">Errors per segment</label>
          <input
            id="eval-errors"
            inputMode="numeric"
            value={maxErrors}
            onChange={(e) => setMaxErrors(e.target.value)}
          />
        </div>
        <button className="primary" disabled={blockers.length > 0} onClick={start}>
          Start evaluation
        </button>
      </div>
      {blockers.length > 0 && (
        <ul className="hint" style={{ marginTop: 10 }}>
          {blockers.map((b) => (
            <li key={b}>{b}</li>
          ))}
        </ul>
      )}
      {error && <p className="error">{error}</p>}
    </div>
  );
}

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
  /** What kind of session is open, when one is: a recording stops here, the
   *  others stop where they started so the device session ends with them. */
  const openKind = sessions.find((s) => s.session_id === recording)?.kind;

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
        ) : recording && openKind !== "recording" ? (
          <p className="hint">
            A {openKind?.replace(/_/g, " ")} session is running. It is stopped where it
            was started, so that the device session and the recording always end
            together.
          </p>
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

      <Evaluation
        patients={patients}
        running={
          sessions.find((s) => s.session_id === recording && s.kind === "evaluation") ?? null
        }
        refresh={refresh}
      />

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
