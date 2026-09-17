/**
 * The one loud element: device state, link, and sensor health, readable at a
 * glance from across a bench. Everything here is measured, never inferred.
 */
import type { DeviceConfig, Snapshot, Vocabulary } from "../api";

function sensorHealth(snapshot: Snapshot | null, sensor: "foot" | "shank"): string {
  if (!snapshot?.status) return "—";
  const faults = snapshot.faults.filter((f) => f.startsWith(sensor));
  if (faults.length > 0) return faults.map((f) => f.replace(`${sensor}_`, "")).join(", ");
  return "ok";
}

export function StateBar({
  snapshot,
  config,
  vocabulary,
  recording,
}: {
  snapshot: Snapshot | null;
  config: DeviceConfig | null;
  vocabulary: Vocabulary | null;
  recording: string | null;
}) {
  const state = snapshot?.link_state === "connected" ? snapshot.device_state : "disconnected";
  const label = state.replace(/_/g, " ");
  const connecting = snapshot?.link_state === "connecting";
  const status = snapshot?.status;
  const age = snapshot?.status_age_ms;
  const stale = age !== null && age !== undefined && age > 2000;

  return (
    <header className="state-bar">
      <div className="state-block state-primary">
        <span className="label">Device</span>
        <span className="value">
          <span className={`dot ${connecting ? "connecting" : state}`} />
          {connecting ? "connecting" : label}
        </span>
      </div>

      <div className="state-block">
        <span className="label">Link</span>
        <span className="value">
          {snapshot?.link_state === "connected" ? (
            <>
              {snapshot.link_description}
              {stale && <span className="fault-list"> no status for {Math.round((age ?? 0) / 1000)} s</span>}
            </>
          ) : (
            <span className="absent">not connected</span>
          )}
        </span>
      </div>

      <div className="state-block">
        <span className="label">Sensors</span>
        <span className="value" style={{ display: "flex", gap: 14, fontSize: 13 }}>
          <span className="sensor foot">
            <span className="name">foot {sensorHealth(snapshot, "foot")}</span>
          </span>
          <span className="sensor shank">
            <span className="name">shank {sensorHealth(snapshot, "shank")}</span>
          </span>
        </span>
      </div>

      <div className="state-block">
        <span className="label">Frames</span>
        <span className="value num">
          {status ? status.frame_index.toLocaleString() : <span className="absent">—</span>}
          {status && status.frames_dropped > 0 && (
            <span className="fault-list"> {status.frames_dropped} dropped</span>
          )}
        </span>
      </div>

      <div className="state-block">
        <span className="label">Recording</span>
        <span className="value">
          {recording ? (
            <span className="num" style={{ color: "var(--critical)" }}>
              ● {recording}
            </span>
          ) : (
            <span className="absent">idle</span>
          )}
        </span>
      </div>

      {snapshot?.faults && snapshot.faults.length > 0 && (
        <div className="state-block">
          <span className="label">Faults</span>
          <span className="value fault-list">{snapshot.faults.join(", ")}</span>
        </div>
      )}

      <div className="state-spacer" />

      <div className="state-block">
        <span className="label">Haptics</span>
        <span className="value absent" title="No ERM driver channels are fitted (DEC-006)">
          not fitted
        </span>
      </div>

      <div className="state-block">
        <span className="label">Firmware</span>
        <span className="value num" style={{ fontSize: 13 }}>
          {snapshot?.firmware ?? <span className="absent">—</span>}
        </span>
      </div>
      {vocabulary && config === null && snapshot?.link_state === "connected" && (
        <div className="state-block">
          <span className="label">Configuration</span>
          <span className="value absent">loading</span>
        </div>
      )}
    </header>
  );
}
