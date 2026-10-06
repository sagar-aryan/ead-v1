#!/usr/bin/env python3
"""Check an exported session.mat against the CSV files beside it.

The .mat writer is hand-rolled (DEC-003), so nothing in this project would
notice if it produced bytes MATLAB refuses. This reads it with scipy, which is
an independent implementation of the Level-5 format, and then checks that what
came out agrees with raw.csv, gait.csv and metadata.json — the same data by a
different route.

Run:  python3 tools/check_mat.py <export directory>
"""
import csv
import json
import pathlib
import sys

import numpy as np
from scipy.io import loadmat


def field(struct, name):
    """One field of a 1x1 struct, unwrapped from scipy's nesting."""
    return struct[name][0, 0]


def text(value):
    return str(np.atleast_1d(value).ravel()[0]) if value.size else ""


def main(directory):
    directory = pathlib.Path(directory)
    mat = loadmat(directory / "session.mat")
    failures = []

    def check(condition, message):
        print(f"{'ok  ' if condition else 'FAIL'}  {message}")
        if not condition:
            failures.append(message)

    for name in ("raw", "gait", "events", "haptics", "metadata", "reference"):
        check(name in mat, f"top-level variable `{name}` present (doc 10 §7)")
    if failures:
        return 1

    # ---- raw: integers, and the same ones raw.csv has ----------------------
    raw = mat["raw"]
    values = field(raw, "values")
    timestamps = field(raw, "timestamp_us")
    check(values.dtype == np.int32, f"raw.values is int32, not {values.dtype}")
    check(timestamps.dtype == np.int64, f"raw.timestamp_us is int64, not {timestamps.dtype}")

    with open(directory / "raw.csv", newline="") as fh:
        rows = list(csv.DictReader(fh))
    check(values.shape[0] == len(rows), f"raw rows {values.shape[0]} == raw.csv {len(rows)}")
    if rows:
        columns = [str(c[0]) for c in field(raw, "columns").ravel()]
        first = rows[0]
        check(int(timestamps[0, 0]) == int(first["timestamp_us"]), "first timestamp matches")
        for i, column in enumerate(columns):
            if column == "sensor":
                expected = 0 if first["sensor"] == "foot" else 1
            else:
                expected = int(first[column])
            check(int(values[0, i]) == expected, f"raw first row {column} = {expected}")

    # ---- gait: NaN where the CSV is empty, numbers where it is not ---------
    gait = mat["gait"]
    gvalues = field(gait, "values")
    gcolumns = [str(c[0]) for c in field(gait, "columns").ravel()]
    with open(directory / "gait.csv", newline="") as fh:
        grows = list(csv.DictReader(fh))
    check(gvalues.shape[0] == len(grows), f"gait rows {gvalues.shape[0]} == gait.csv {len(grows)}")
    pairs = [("cycle_time_s", "cycle_time_s"), ("cycle_distance_m", "cycle_distance_m"),
             ("unilateral_cycle_symmetry_proxy", "unilateral_symmetry_proxy"),
             ("error_score", "error_score"), ("confidence", "confidence"), ("valid", "valid")]
    for mat_name, csv_name in pairs:
        if mat_name not in gcolumns:
            continue
        column = gvalues[:, gcolumns.index(mat_name)]
        for row_index, row in enumerate(grows):
            cell = row[csv_name].strip()
            value = column[row_index]
            if cell == "":
                check(np.isnan(value), f"{mat_name} row {row_index + 1} is NaN where the CSV is empty")
            else:
                check(abs(value - float(cell)) < 1e-3,
                      f"{mat_name} row {row_index + 1} = {cell}")

    # ---- metadata: agrees with metadata.json -------------------------------
    meta = mat["metadata"]
    with open(directory / "metadata.json") as fh:
        js = json.load(fh)
    check(text(field(meta, "session_id")) == js["session"]["session_id"], "session_id matches")
    check(text(field(meta, "patient_id")) == js["patient"]["patient_id"], "patient_id matches")
    name = js["patient"]["patient_name"]
    check(text(field(meta, "patient_name")) == name, "patient_name matches")
    # DEC-028: every CSV carries the patient's name on every row.
    for csv_name in ("raw.csv", "accel_native.csv", "gait.csv", "events.csv", "haptics.csv"):
        with open(directory / csv_name, newline="") as fh:
            names = {row["patient_name"] for row in csv.DictReader(fh)}
        check(names <= {name}, f"{csv_name} names only {name!r}")
    check(int(field(meta, "protocol_version")[0, 0]) == js["device"]["protocol_version"],
          "protocol_version matches")

    # ---- haptics: the same records haptics.csv has (DEC-023) ---------------
    # Until 2026-10-05 this expected the empty, "no ERM drivers are fitted" struct
    # of DEC-006, and failed on every export after the drivers were fitted.
    haptics = mat["haptics"]
    hvalues = field(haptics, "values")
    hcolumns = [text(c) for c in field(haptics, "columns").ravel()]
    with open(directory / "haptics.csv", newline="") as fh:
        hrows = list(csv.DictReader(fh))
    count = hvalues.shape[0] if hvalues.size else 0
    check(count == len(hrows), f"haptics rows {count} == haptics.csv {len(hrows)}")
    for i, row in enumerate(hrows[:count]):
        mrow = dict(zip(hcolumns, hvalues[i]))
        motors = [int(m) for m in row["motor_ids"].split(";") if m]
        duties = [int(d) for d in row["pwm"].split(";") if d]
        check(mrow["timestamp_us"] == int(row["timestamp_us"])
              and [m for m in (mrow["motor_a"], mrow["motor_b"]) if m] == motors
              and [d for d in (mrow["duty_a"], mrow["duty_b"]) if d] == duties
              and mrow["duration_ms"] == int(row["duration_ms"]),
              f"haptic record {i}: time, motors, duties and length match haptics.csv")
        scores = field(haptics, "scores")[i]
        check(abs(scores[0] - float(row["error_score"])) < 1e-6
              and abs(scores[1] - float(row["confidence"])) < 1e-6,
              f"haptic record {i}: score and confidence match")
        check(text(field(haptics, "event").ravel()[i]) == row["event"]
              and text(field(haptics, "reason").ravel()[i]) == row["reason"],
              f"haptic record {i}: event and reason match")

    print(f"\n{len(failures)} failures")
    return 1 if failures else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        raise SystemExit(2)
    raise SystemExit(main(sys.argv[1]))
