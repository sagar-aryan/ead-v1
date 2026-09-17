/**
 * Mounting check: does each sensor sit on the leg the way the firmware's mount
 * map assumes?
 *
 * The anatomical frame is X forward, Y medial, Z up (right leg, right-handed).
 * Standing still, gravity must read as +Z on both sensors. Raising the toes and
 * extending the knee both rotate the segment about the medial axis in the
 * negative sense, so each move must show up as a negative Y angular rate on the
 * segment that moved. Any other dominant axis means the mount map and the real
 * mounting disagree — which is what PROB-002 is about, so this reports the
 * measured numbers rather than only a verdict.
 */

export type Vec3 = [number, number, number];

export const AXIS_NAMES = ["X", "Y", "Z"] as const;

/** Standing still: gravity along +Z, with the other axes near zero. */
export const STILL_UP_G = 0.9;
export const STILL_TILT_G = 0.3;
/** |a| away from 1 g means the sensor is moving or the scale is wrong. */
export const STILL_MAGNITUDE_TOLERANCE_G = 0.12;
/** Below this the move was too gentle to tell the axes apart. */
export const MIN_RATE_DPS = 30;
/** The intended axis must lead the next one by this factor. */
export const MIN_DOMINANCE = 1.5;

export interface StillVerdict {
  pass: boolean;
  mean: Vec3;
  magnitude: number;
  reason: string;
}

export interface RotationVerdict {
  pass: boolean;
  peak: Vec3;
  /** Axis with the largest rate, and its sign at that moment. */
  axis: (typeof AXIS_NAMES)[number];
  rate: number;
  dominance: number;
  reason: string;
}

const mean = (samples: Vec3[], axis: number) =>
  samples.reduce((total, s) => total + s[axis], 0) / samples.length;

/** Mean acceleration over a still window, checked against gravity along +Z. */
export function stillVerdict(samples: Vec3[]): StillVerdict {
  if (samples.length === 0) {
    return { pass: false, mean: [0, 0, 0], magnitude: 0, reason: "no samples" };
  }
  const m: Vec3 = [mean(samples, 0), mean(samples, 1), mean(samples, 2)];
  const magnitude = Math.hypot(...m);
  if (Math.abs(magnitude - 1) > STILL_MAGNITUDE_TOLERANCE_G) {
    return { pass: false, mean: m, magnitude, reason: `|a| is ${magnitude.toFixed(2)} g, not 1 g` };
  }
  if (m[2] < STILL_UP_G) {
    const dominant = m.indexOf(m.reduce((a, b) => (Math.abs(a) > Math.abs(b) ? a : b)));
    return {
      pass: false,
      mean: m,
      magnitude,
      reason: `gravity reads on ${AXIS_NAMES[dominant]} (${m[dominant].toFixed(2)} g), not +Z`,
    };
  }
  if (Math.abs(m[0]) > STILL_TILT_G || Math.abs(m[1]) > STILL_TILT_G) {
    return { pass: false, mean: m, magnitude, reason: "the sensor is tilted more than expected" };
  }
  return { pass: true, mean: m, magnitude, reason: "gravity reads +Z" };
}

/**
 * The strongest moment of a guided movement, checked against the axis and sign
 * the mount map implies.
 *
 * Judged on the single sample with the largest rate on any axis rather than on
 * an average, because the move is short and the operator's timing is not.
 */
export function rotationVerdict(
  samples: Vec3[],
  expectedAxis: 0 | 1 | 2,
  expectedSign: -1 | 1,
): RotationVerdict {
  const empty: RotationVerdict = {
    pass: false,
    peak: [0, 0, 0],
    axis: "X",
    rate: 0,
    dominance: 0,
    reason: "no samples",
  };
  if (samples.length === 0) return empty;

  let peak: Vec3 = [0, 0, 0];
  let best = 0;
  for (const sample of samples) {
    const strength = Math.max(...sample.map(Math.abs));
    if (strength > best) {
      best = strength;
      peak = sample;
    }
  }
  const sorted = [...peak].map(Math.abs).sort((a, b) => b - a);
  const axis = peak.findIndex((v) => Math.abs(v) === sorted[0]);
  const rate = peak[axis];
  const dominance = sorted[1] === 0 ? Infinity : sorted[0] / sorted[1];
  const verdict = { peak, axis: AXIS_NAMES[axis], rate, dominance };

  if (sorted[0] < MIN_RATE_DPS) {
    return { ...verdict, pass: false, reason: `only ${sorted[0].toFixed(0)} °/s: move further` };
  }
  if (axis !== expectedAxis) {
    return {
      ...verdict,
      pass: false,
      reason: `turned about ${AXIS_NAMES[axis]}, expected ${AXIS_NAMES[expectedAxis]}`,
    };
  }
  if (Math.sign(rate) !== expectedSign) {
    return {
      ...verdict,
      pass: false,
      reason: `${AXIS_NAMES[axis]} rate is ${rate > 0 ? "positive" : "negative"}, expected the opposite sign`,
    };
  }
  if (dominance < MIN_DOMINANCE) {
    return { ...verdict, pass: false, reason: "the movement was not clearly about one axis" };
  }
  return { ...verdict, pass: true, reason: `${rate.toFixed(0)} °/s about ${AXIS_NAMES[axis]}` };
}
