/**
 * Window arithmetic for the recorded-data views.
 *
 * A reader moves through a recording by frame index, not by time: frame indices
 * are what the store is keyed on, and the device's sample period is not exactly
 * 10 ms (its oscillator runs at 100.145 Hz, TEST-018), so deriving one from the
 * other would drift. These functions are pure and tested; the views hold the
 * state and draw.
 */

/** Frames. A narrower window holds too few samples to read anything from. */
export const MIN_SPAN = 20;

/** Inclusive frame range of a recorded session. */
export interface Bounds {
  first: number;
  last: number;
}

/**
 * Index of the value nearest `target` in an ascending array.
 *
 * Takes the array rather than a window because `time_s` and `frame_index` are
 * parallel: an index found in one reads straight out of the other.
 */
export function nearest(values: number[], target: number): number {
  if (values.length === 0) return -1;
  let lo = 0;
  let hi = values.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (values[mid] < target) lo = mid + 1;
    else hi = mid;
  }
  if (lo > 0 && target - values[lo - 1] < values[lo] - target) return lo - 1;
  return lo;
}

/**
 * Places a window of `span` frames starting at `from`, keeping it inside the
 * session and keeping its width.
 *
 * Returns null when the window covers the whole session, so that "whole
 * session" has exactly one representation: a caller storing the result cannot
 * end up in a state that looks zoomed while showing everything.
 */
export function placeWindow(
  bounds: Bounds,
  from: number,
  span: number,
): [number, number] | null {
  const total = bounds.last - bounds.first + 1;
  const width = Math.min(total, Math.max(MIN_SPAN, Math.round(span)));
  if (width >= total) return null;
  const start = Math.min(Math.max(Math.round(from), bounds.first), bounds.last - width + 1);
  return [start, start + width - 1];
}

/** Elapsed device time as m:ss.s, which is how a reader refers to a moment. */
export function clock(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${(seconds - minutes * 60).toFixed(1).padStart(4, "0")}`;
}
