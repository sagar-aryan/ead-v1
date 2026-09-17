/**
 * Pairing for the events view. Kept out of the component so it can be tested
 * without a DOM, and because getting the pairing wrong is the failure that
 * would make a zero-velocity lane quietly lie.
 */
import type { GaitEvent } from "./api";

/** A device-time interval, µs. */
export interface Span {
  from: number;
  to: number;
}

/**
 * Pairs each `zupt_start` with the next `zupt_end`.
 *
 * A second start before an end is ignored rather than opening a nested window:
 * the detector cannot be in two zero-velocity windows at once, so the extra
 * start is a duplicate, not a new interval. An `end` with nothing open is
 * dropped for the same reason. A window still open when the session ended is
 * drawn to `endUs`, because a ZUPT that never closed is exactly what a reader
 * needs to see.
 */
export function zuptSpans(events: GaitEvent[], endUs: number): Span[] {
  const spans: Span[] = [];
  let open: number | null = null;
  for (const e of events) {
    if (e.kind === "zupt_start" && open === null) open = e.timestamp_us;
    else if (e.kind === "zupt_end" && open !== null) {
      spans.push({ from: open, to: e.timestamp_us });
      open = null;
    }
  }
  if (open !== null) spans.push({ from: open, to: endUs });
  return spans;
}
