/**
 * Live orientation: the two segments drawn from the side, with the three ankle
 * angles the device's quaternions imply.
 *
 * Drawn on a canvas rather than with a 3D library: two segments in the sagittal
 * plane is a line and a wedge, and a reader learns more from a shape that
 * matches the numbers beside it than from a rotating model.
 *
 * The panel says "not available" whenever the device reports no orientation —
 * before calibration, or after a timing gap — instead of drawing a neutral
 * ankle, which would look like a measurement of a straight foot.
 */
import { useEffect, useRef } from "react";

import type { AnkleAngles } from "../api";
import type { DeviceApi } from "../useDevice";

const INK = "#15181c";
const FOOT = "#2a78d6";
const SHANK = "#eb6834";
const RULE = "#e3e4e2";
const HEIGHT = 190;

/**
 * Side view: the shank as a line from the knee down to the ankle, the foot as a
 * line from the ankle forward. Only the sagittal angle can be shown this way;
 * the other two are given as numbers.
 */
function draw(canvas: HTMLCanvasElement, sagittal: number | null) {
  const ratio = window.devicePixelRatio || 1;
  const width = canvas.clientWidth;
  canvas.width = width * ratio;
  canvas.height = HEIGHT * ratio;
  const c = canvas.getContext("2d");
  if (!c) return;
  c.setTransform(ratio, 0, 0, ratio, 0, 0);
  c.clearRect(0, 0, width, HEIGHT);

  const ankleX = width / 2 - 20;
  const ankleY = HEIGHT - 45;
  const shankLength = HEIGHT - 80;
  const footLength = 74;

  // Ground line, for the eye to judge the foot against.
  c.strokeStyle = RULE;
  c.lineWidth = 1;
  c.beginPath();
  c.moveTo(16, ankleY + 26);
  c.lineTo(width - 16, ankleY + 26);
  c.stroke();

  // Shank: vertical, since the drawing is relative to it.
  c.strokeStyle = SHANK;
  c.lineWidth = 5;
  c.lineCap = "round";
  c.beginPath();
  c.moveTo(ankleX, ankleY - shankLength);
  c.lineTo(ankleX, ankleY);
  c.stroke();

  if (sagittal === null) {
    c.fillStyle = "#868c93";
    c.font = '12px "IBM Plex Sans", system-ui, sans-serif';
    c.fillText("orientation not available", ankleX + 12, ankleY - shankLength / 2);
    return;
  }

  // Neutral is the foot at a right angle to the shank; positive dorsiflexion
  // lifts the toes, so it rotates the foot upward on screen.
  const angle = (-sagittal * Math.PI) / 180;
  c.strokeStyle = FOOT;
  c.beginPath();
  c.moveTo(ankleX, ankleY);
  c.lineTo(ankleX + footLength * Math.cos(angle), ankleY + footLength * Math.sin(angle));
  c.stroke();

  // The neutral position, so the drawn angle can be read against it.
  c.strokeStyle = RULE;
  c.lineWidth = 1;
  c.setLineDash([3, 3]);
  c.beginPath();
  c.moveTo(ankleX, ankleY);
  c.lineTo(ankleX + footLength, ankleY);
  c.stroke();
  c.setLineDash([]);

  c.fillStyle = INK;
  c.font = '12px "IBM Plex Mono", monospace';
  c.fillText(`${sagittal >= 0 ? "+" : ""}${sagittal.toFixed(1)}°`, ankleX + footLength + 8, ankleY);
}

function Angle({ label, value, note }: { label: string; value: number | null; note: string }) {
  return (
    <div className="readout">
      <span className="label">{label}</span>
      <span className="value num">
        {value === null ? (
          <span className="absent">—</span>
        ) : (
          <>
            {value >= 0 ? "+" : ""}
            {value.toFixed(1)}
            <span className="unit">°</span>
          </>
        )}
      </span>
      <span className="label" style={{ marginTop: 4 }}>
        {note}
      </span>
    </div>
  );
}

export function Orientation({ device }: { device: DeviceApi }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const angles: AnkleAngles | null = device.tick?.orientation?.angles ?? null;
  const sagittal = angles?.sagittal_deg ?? null;

  useEffect(() => {
    if (canvas.current) draw(canvas.current, sagittal);
  }, [sagittal]);

  useEffect(() => {
    const redraw = () => canvas.current && draw(canvas.current, sagittal);
    window.addEventListener("resize", redraw);
    return () => window.removeEventListener("resize", redraw);
  }, [sagittal]);

  return (
    <div className="panel">
      <h2>Orientation</h2>
      <p className="hint">
        The foot relative to the shank, which is the measurement that does not
        depend on which way the patient is facing. These are segment angles from
        two IMUs, not joint goniometry.
      </p>
      {!device.tick?.orientation && (
        <p className="empty">
          The device reports no orientation. Calibrate it in the Device view.
        </p>
      )}
      <canvas ref={canvas} style={{ width: "100%", height: HEIGHT }} />
      <div className="readouts" style={{ marginTop: 12 }}>
        <Angle label="Sagittal" value={angles?.sagittal_deg ?? null} note="+ dorsiflexion-related" />
        <Angle label="Frontal" value={angles?.frontal_deg ?? null} note="+ inversion-related" />
        <Angle
          label="Transverse"
          value={angles?.transverse_deg ?? null}
          note="drifts: no heading reference"
        />
      </div>
    </div>
  );
}
