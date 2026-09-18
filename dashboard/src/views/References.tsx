/**
 * REFERENCES: the patient's own walking, captured once and then held still.
 *
 * Doc 12 §2–§3. Every patient is compared with a profile built from at least
 * thirty of their own valid cycles, never with a healthy-population pattern.
 * The device builds the profile (DEC-012); this view versions it, shows what is
 * in it, and locks it the moment it has judged a session — after that the
 * numbers on screen are exactly the numbers that produced every score made
 * against them.
 *
 * A capture and a check both run inside a recording session, so the walk that
 * produced a profile is still on disk and can be replayed against a corrected
 * detector later.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  api,
  type Cycle,
  type Patient,
  type StoredReference,
  type Vocabulary,
} from "../api";
import type { DeviceApi } from "../useDevice";

/** Units for each feature, in `Vocabulary.feature_names` order (doc 06 §3). */
const FEATURE_UNITS = ["°", "°", "°", "s", "", "m", "°/s"];
const FEATURE_DIGITS = [1, 1, 1, 3, 3, 3, 0];

/** Feature names read as identifiers; these are how doc 06 says them. */
const FEATURE_LABELS: Record<string, string> = {
  swing_dorsiflexion: "Swing dorsiflexion",
  contact_plantarflexion: "Contact plantarflexion",
  inversion: "Peak inversion",
  cycle_time: "Cycle time",
  stance_ratio: "Stance ratio",
  cycle_distance: "Cycle distance",
  shank_dynamics: "Peak shank rate",
};

export function featureLabel(name: string): string {
  return FEATURE_LABELS[name] ?? name;
}

export function className(vocabulary: Vocabulary | null, index: number): string {
  const raw = vocabulary?.error_classes[index] ?? String(index);
  return raw.replace(/_/g, " ");
}

function median(values: number[]): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

/** What the device collected for a profile, or what a check has read so far. */
function Progress({ have, need, noun }: { have: number; need: number; noun: string }) {
  const done = Math.min(have, need);
  return (
    <div className="readout wide">
      <span className="label">
        {noun} ({done} of {need})
      </span>
      <span className="value num">
        <span
          aria-hidden
          style={{
            display: "inline-block",
            width: 180,
            height: 8,
            background: "#eceeed",
            verticalAlign: "middle",
            marginRight: 10,
          }}
        >
          <span
            style={{
              display: "block",
              width: `${(done / need) * 100}%`,
              height: "100%",
              background: "var(--ink, #15181c)",
            }}
          />
        </span>
        {have}
      </span>
    </div>
  );
}

export function References({ device }: { device: DeviceApi }) {
  const [patients, setPatients] = useState<Patient[]>([]);
  const [patientId, setPatientId] = useState("");
  const [references, setReferences] = useState<StoredReference[]>([]);
  const [selected, setSelected] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  /** The session this view started, so it stops the one it owns. */
  const [running, setRunning] = useState<{ sessionId: string; check: boolean } | null>(null);
  const [checkCycles, setCheckCycles] = useState<Cycle[]>([]);

  const snapshot = device.snapshot;
  const vocabulary = device.vocabulary;
  const minCycles = device.config?.reference_min_cycles ?? 30;
  const checkTarget = device.config?.reference_check_cycles ?? 10;

  /** A check's reference to select once the patient's versions have loaded. */
  const pendingSelection = useRef<string | null>(null);

  useEffect(() => {
    api.patients().then(setPatients).catch((e) => setError(String(e)));
  }, []);

  // The session lives in the backend, not in this page: leaving the tab and
  // coming back must pick a running capture or check back up, not offer to
  // start another beside it.
  useEffect(() => {
    (async () => {
      const open = await api.recordingSession();
      if (!open) return;
      const session = await api.session(open);
      if (session.kind !== "reference_capture" && session.kind !== "reference_check") return;
      const check = session.kind === "reference_check";
      pendingSelection.current = check ? session.reference_id : null;
      setPatientId(session.patient_id);
      setRunning({ sessionId: open, check });
    })().catch((e) => setError(String(e)));
  }, []);

  const loadReferences = useCallback(async (id: string) => {
    if (!id) {
      setReferences([]);
      return;
    }
    try {
      setReferences(await api.references(id));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    setSelected("");
    loadReferences(patientId).then(() => {
      if (pendingSelection.current) setSelected(pendingSelection.current);
      pendingSelection.current = null;
    });
  }, [patientId, loadReferences]);

  // While a check runs, read back what the device has scored so far.
  useEffect(() => {
    if (!running?.check) return;
    const poll = () =>
      api.cycles(running.sessionId).then(setCheckCycles).catch(() => undefined);
    poll();
    const timer = setInterval(poll, 1000);
    return () => clearInterval(timer);
  }, [running]);

  const guard = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const startCapture = () =>
    guard(async () => {
      setNote(null);
      const session = await api.startReferenceCapture(patientId);
      setRunning({ sessionId: session.session_id, check: false });
    });

  const finishCapture = () =>
    guard(async () => {
      const saved = await api.finishReferenceCapture(patientId);
      setRunning(null);
      setNote(
        saved
          ? `Saved version ${saved.version} from ${saved.cycles} valid cycles.`
          : `The device built no profile: it refuses to make one from fewer than ` +
            `${minCycles} valid cycles. The walk itself is still recorded.`,
      );
      await loadReferences(patientId);
    });

  const startCheck = () =>
    guard(async () => {
      setNote(null);
      setCheckCycles([]);
      const session = await api.startScoredSession(patientId, selected, true, null);
      setRunning({ sessionId: session.session_id, check: true });
    });

  const stopCheck = () =>
    guard(async () => {
      await api.stopScoredSession();
      setRunning(null);
      await loadReferences(patientId);
    });

  const profile = references.find((r) => r.reference_id === selected);
  const scored = useMemo(
    () => checkCycles.filter((c) => c.valid && c.confidence > 0),
    [checkCycles],
  );
  const medianScore = median(scored.map((c) => c.error_score));
  const medianConfidence = median(scored.map((c) => c.confidence));

  const capturing = running !== null && !running.check;
  const checking = running !== null && running.check;

  return (
    <>
      <h1>References</h1>
      <p className="page-hint">
        A reference profile is the patient's own walking — the median and spread of
        each feature over at least {minCycles} of their valid cycles. It is never a
        healthy-population pattern, and once it has judged a session it cannot be
        changed.
      </p>

      <div className="panel">
        <div className="row">
          <div className="field">
            <label htmlFor="ref-patient">Patient</label>
            <select
              id="ref-patient"
              value={patientId}
              disabled={running !== null}
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
        </div>
        {error && <p className="error">{error}</p>}
        {note && <p className="hint">{note}</p>}
      </div>

      {patientId && (
        <div className="panel">
          <h2>Capture</h2>
          <p className="hint">
            The patient walks; the device keeps every valid cycle and builds the
            profile itself when you stop. Fewer than {minCycles} valid cycles and it
            builds nothing — a profile resting on less evidence than that would make
            every later score look better founded than it is.
          </p>
          {capturing ? (
            <>
              <div className="readouts">
                <Progress
                  have={snapshot?.session_valid_cycles ?? 0}
                  need={minCycles}
                  noun="Valid cycles"
                />
                <div className="readout">
                  <span className="label">Recording</span>
                  <span className="value num">{running.sessionId}</span>
                </div>
              </div>
              <div className="row" style={{ marginTop: 12 }}>
                <button className="primary" disabled={busy} onClick={finishCapture}>
                  Stop and build profile
                </button>
              </div>
            </>
          ) : (
            <div className="row">
              <button className="primary" disabled={busy || checking} onClick={startCapture}>
                Start capture walk
              </button>
              <p className="hint" style={{ margin: 0 }}>
                Needs a connected, calibrated device and no other session running.
              </p>
            </div>
          )}
        </div>
      )}

      {patientId && (
        <div className="panel">
          <h2>Versions</h2>
          {references.length === 0 ? (
            <p className="empty">No reference profile for this patient yet.</p>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>Version</th>
                  <th>Captured</th>
                  <th>Cycles</th>
                  <th>From session</th>
                  <th>State</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {references.map((r) => (
                  <tr key={r.reference_id} aria-selected={r.reference_id === selected}>
                    <td className="num">{r.version}</td>
                    <td className="num">{r.created_at.slice(0, 19).replace("T", " ")}</td>
                    <td className="num">{r.cycles}</td>
                    <td className="num" style={{ fontSize: 12 }}>
                      {r.session_id ?? <span className="absent">—</span>}
                    </td>
                    <td>{r.locked ? "locked" : "unused"}</td>
                    <td>
                      <button onClick={() => setSelected(r.reference_id)}>Show</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {profile && (
        <div className="panel">
          <h2>Version {profile.version}</h2>
          <p className="hint">
            {profile.locked
              ? "Locked: this profile has judged a session, so it is read-only. To " +
                "change what the patient is compared with, capture a new version."
              : "Not yet used. It locks the moment a check or an evaluation runs " +
                "against it."}
          </p>
          <table>
            <thead>
              <tr>
                <th>Feature</th>
                <th>Median</th>
                <th>Spread (1.4826 × MAD)</th>
                <th>Weight</th>
              </tr>
            </thead>
            <tbody>
              {profile.profile.features.map((f, i) => (
                <tr key={i}>
                  <td>{featureLabel(vocabulary?.feature_names[i] ?? String(i))}</td>
                  <td className="num">
                    {f.median.toFixed(FEATURE_DIGITS[i])}
                    <span className="unit">{FEATURE_UNITS[i]}</span>
                  </td>
                  <td className="num">
                    {f.spread.toFixed(FEATURE_DIGITS[i])}
                    <span className="unit">{FEATURE_UNITS[i]}</span>
                  </td>
                  <td className="num">{FEATURE_WEIGHTS[i].toFixed(2)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {profile && (
        <div className="panel">
          <h2>Check</h2>
          <p className="hint">
            {checkTarget} cycles read against version {profile.version}, with no
            haptics (doc 12 §3). This reports agreement; it does not pass or fail the
            patient, because no threshold for that is specified.
          </p>
          {checking ? (
            <>
              <div className="readouts">
                <Progress have={scored.length} need={checkTarget} noun="Scored cycles" />
                <div className="readout">
                  <span className="label">Median error score</span>
                  <span className="value num">
                    {medianScore === null ? <span className="absent">—</span> : medianScore.toFixed(2)}
                  </span>
                </div>
                <div className="readout">
                  <span className="label">Median confidence</span>
                  <span className="value num">
                    {medianConfidence === null ? (
                      <span className="absent">—</span>
                    ) : (
                      medianConfidence.toFixed(2)
                    )}
                  </span>
                </div>
              </div>
              {scored.length > 0 && (
                <table style={{ marginTop: 12 }}>
                  <thead>
                    <tr>
                      <th>Feature</th>
                      <th>Median deviation</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(vocabulary?.feature_names ?? []).map((name, i) => {
                      // A feature dropped for every cycle has no deviation to
                      // report; showing 0 would read as perfect agreement.
                      const values = scored
                        .map((c) => c.deviations[i])
                        .filter((v) => v !== undefined);
                      const d = median(values);
                      return (
                        <tr key={name}>
                          <td>{featureLabel(name)}</td>
                          <td className="num">
                            {d === null ? (
                              <span className="absent">not measured</span>
                            ) : (
                              d.toFixed(2)
                            )}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              )}
              <div className="row" style={{ marginTop: 12 }}>
                <button className="danger" disabled={busy} onClick={stopCheck}>
                  Stop check
                </button>
              </div>
            </>
          ) : (
            <div className="row">
              <button className="primary" disabled={busy || capturing} onClick={startCheck}>
                Start check walk
              </button>
              {!profile.locked && (
                <p className="hint" style={{ margin: 0 }}>
                  This locks version {profile.version}.
                </p>
              )}
            </div>
          )}
        </div>
      )}
    </>
  );
}

/** Doc 06 §3, mirrored from `ead::kFeatureWeights` for display only. */
const FEATURE_WEIGHTS = [0.25, 0.15, 0.15, 0.15, 0.1, 0.1, 0.1];
