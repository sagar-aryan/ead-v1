/**
 * Tests for the window arithmetic behind the raw-data navigation.
 *
 * Run with `npm --prefix dashboard test`. Node runs the TypeScript directly, so
 * this needs no test framework and no new dependency; keep the syntax erasable
 * (no enums, no parameter properties) or Node will refuse to strip the types.
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import { MIN_SPAN, clock, nearest, placeWindow } from "./timeline.ts";

// The imported 30-minute bench recording (TEST-018).
const SESSION = { first: 0, last: 180_249 };

test("nearest finds the closest point, not just the one after", () => {
  const times = [0, 1, 2, 3, 4];
  assert.equal(nearest(times, 2), 2);
  assert.equal(nearest(times, 2.4), 2);
  assert.equal(nearest(times, 2.6), 3);
  // Outside the range on either side clamps to an end rather than throwing.
  assert.equal(nearest(times, -50), 0);
  assert.equal(nearest(times, 50), 4);
  assert.equal(nearest([], 1), -1);
});

test("nearest works on the uneven spacing of a decimated window", () => {
  // Buckets of a long window: times are device timestamps, not a fixed grid.
  const times = [0, 1.278, 2.556, 3.835, 5.113];
  assert.equal(nearest(times, 3.8), 3);
  assert.equal(nearest(times, 1.9), 1);
  assert.equal(nearest(times, 1.92), 2);
});

test("a window keeps its width when it runs into the start of the session", () => {
  const window = placeWindow(SESSION, -500, 1000);
  assert.deepEqual(window, [0, 999]);
});

test("a window keeps its width when it runs into the end of the session", () => {
  const window = placeWindow(SESSION, 180_000, 1000);
  // Not [180000, 180999]: that would ask for frames the session does not have.
  assert.deepEqual(window, [179_250, 180_249]);
});

test("a window is never narrower than MIN_SPAN", () => {
  const window = placeWindow(SESSION, 1000, 1);
  assert.ok(window);
  assert.equal(window[1] - window[0] + 1, MIN_SPAN);
});

test("covering the whole session is reported as null, not as a zoom", () => {
  assert.equal(placeWindow(SESSION, 0, 180_250), null);
  // Asking for more than exists is still the whole session.
  assert.equal(placeWindow(SESSION, -1000, 999_999), null);
});

test("panning right by half a window repeatedly stops at the last frame", () => {
  let window: [number, number] | null = [0, 999];
  for (let step = 0; step < 1000; step += 1) {
    const span = window[1] - window[0] + 1;
    const next = placeWindow(SESSION, window[0] + Math.round(span / 2), span);
    assert.ok(next, "a 1000-frame window is never the whole session");
    window = next;
    assert.ok(window[0] >= SESSION.first && window[1] <= SESSION.last, "stayed inside");
  }
  assert.deepEqual(window, [179_250, 180_249]);
});

test("zooming out doubles the width until it is the whole session", () => {
  let window: [number, number] | null = [90_000, 90_999];
  let steps = 0;
  while (window && steps < 20) {
    const span = window[1] - window[0] + 1;
    const centre = (window[0] + window[1]) / 2;
    window = placeWindow(SESSION, centre - span, span * 2);
    steps += 1;
  }
  assert.equal(window, null, "zooming out reaches the whole session");
  assert.equal(steps, 8, "1000 frames doubles to 180,250 in eight steps");
});

test("a session shorter than MIN_SPAN is always the whole session", () => {
  assert.equal(placeWindow({ first: 0, last: 5 }, 2, 2), null);
});

test("clock reads as elapsed minutes and seconds", () => {
  assert.equal(clock(0), "0:00.0");
  assert.equal(clock(9.25), "0:09.3");
  assert.equal(clock(60), "1:00.0");
  // The moment in the screenshot that started this: 899.5 s into the recording.
  assert.equal(clock(899.5), "14:59.5");
  assert.equal(clock(1800), "30:00.0");
});
