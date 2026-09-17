import assert from "node:assert/strict";
import { test } from "node:test";

import { zuptSpans } from "./events.ts";

const at = (kind: string, us: number) => ({ kind, timestamp_us: us, frame_index: us / 10_000 });

test("pairs each start with the next end", () => {
  const spans = zuptSpans(
    [at("zupt_start", 1000), at("zupt_end", 2000), at("zupt_start", 5000), at("zupt_end", 6000)],
    9000,
  );
  assert.deepEqual(spans, [
    { from: 1000, to: 2000 },
    { from: 5000, to: 6000 },
  ]);
});

test("ignores events of other kinds", () => {
  const spans = zuptSpans(
    [at("zupt_start", 1000), at("foot_flat", 1500), at("zupt_end", 2000)],
    9000,
  );
  assert.deepEqual(spans, [{ from: 1000, to: 2000 }]);
});

test("a repeated start does not open a second window", () => {
  const spans = zuptSpans([at("zupt_start", 1000), at("zupt_start", 1200), at("zupt_end", 2000)], 9000);
  assert.deepEqual(spans, [{ from: 1000, to: 2000 }]);
});

test("an end with nothing open is dropped", () => {
  assert.deepEqual(zuptSpans([at("zupt_end", 2000)], 9000), []);
});

test("a window still open at the end of the session runs to the end", () => {
  assert.deepEqual(zuptSpans([at("zupt_start", 8000)], 9000), [{ from: 8000, to: 9000 }]);
});
