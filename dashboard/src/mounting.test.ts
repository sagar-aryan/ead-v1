/**
 * Tests for the mounting check verdicts. Run with `npm --prefix dashboard test`.
 *
 * The failing cases matter more than the passing one: this check exists to catch
 * a mount map that disagrees with the real mounting (PROB-002), so it has to
 * fail loudly on the wrong axis and on the wrong sign.
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import { type Vec3, rotationVerdict, stillVerdict, tiltDegrees } from "./mounting.ts";

const repeat = (v: Vec3, n = 60): Vec3[] => Array.from({ length: n }, () => v);

test("standing still passes when gravity reads +Z", () => {
  const verdict = stillVerdict(repeat([0.02, -0.04, 1.0]));
  assert.equal(verdict.pass, true);
  assert.equal(verdict.magnitude.toFixed(2), "1.00");
});

test("a board tilted on the instep passes and reports the angle", () => {
  // Measured on the foot, TEST-027: gravity is on +Z, the strap holds the
  // board at a slope, and calibration's gravity alignment removes that.
  const verdict = stillVerdict(repeat([-0.43, 0.35, 0.86]));
  assert.equal(verdict.pass, true);
  assert.equal(tiltDegrees([-0.43, 0.35, 0.86]), 33);
});

test("a sensor lying on its side still fails", () => {
  assert.equal(stillVerdict(repeat([0.0, 0.94, 0.34])).pass, false);
});

test("gravity on the wrong axis fails and names the axis it found", () => {
  // The case that started PROB-002: the shank reading ~1 g on Y.
  const verdict = stillVerdict(repeat([-0.05, 0.99, -0.06]));
  assert.equal(verdict.pass, false);
  assert.match(verdict.reason, /gravity reads on Y/);
});

test("upside down fails even though the magnitude is right", () => {
  const verdict = stillVerdict(repeat([0, 0, -1]));
  assert.equal(verdict.pass, false);
});

test("a sensor that is not holding still fails on magnitude", () => {
  const verdict = stillVerdict(repeat([0.5, 0.5, 1.2]));
  assert.equal(verdict.pass, false);
  assert.match(verdict.reason, /not 1 g/);
});

test("no samples never passes", () => {
  assert.equal(stillVerdict([]).pass, false);
  assert.equal(rotationVerdict([], 1, -1).pass, false);
});

test("raising the toes passes on a negative Y rate", () => {
  const samples: Vec3[] = [[1, -3, 2], [2, -120, 5], [0, -40, 1]];
  const verdict = rotationVerdict(samples, 1, -1);
  assert.equal(verdict.pass, true);
  assert.equal(verdict.axis, "Y");
  assert.equal(verdict.rate, -120);
});

test("the same movement with the sign reversed fails", () => {
  const verdict = rotationVerdict([[2, 120, 5]], 1, -1);
  assert.equal(verdict.pass, false);
  assert.match(verdict.reason, /expected the opposite sign/);
});

test("a movement about the wrong axis fails and names it", () => {
  const verdict = rotationVerdict([[-150, 10, 4]], 1, -1);
  assert.equal(verdict.pass, false);
  assert.equal(verdict.axis, "X");
  assert.match(verdict.reason, /turned about X, expected Y/);
});

test("a movement too gentle to read fails rather than guessing", () => {
  const verdict = rotationVerdict([[1, -12, 2]], 1, -1);
  assert.equal(verdict.pass, false);
  assert.match(verdict.reason, /move further/);
});

test("a movement spread across two axes fails as unclear", () => {
  const verdict = rotationVerdict([[0, -100, 80]], 1, -1);
  assert.equal(verdict.pass, false);
  assert.match(verdict.reason, /not clearly about one axis/);
});

test("the peak sample is judged, not the average", () => {
  // Long still stretches either side of a brief movement must not dilute it.
  const samples: Vec3[] = [...repeat([0, 0, 0], 40), [0, -90, 3], ...repeat([0, 0, 0], 40)];
  assert.equal(rotationVerdict(samples, 1, -1).pass, true);
});
