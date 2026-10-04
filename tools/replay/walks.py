#!/usr/bin/env python3
"""Replays the 10 m walk fixtures and compares each leg with its ground truth.

  walks.py /path/to/eadreplay [replay flags...]

Per leg: contacts against the right-foot landings the walker counted, and the
summed distance of the valid cycles that start in it. A leg's cycles start at its
first contact, so they cover 10 m less the first step from standing.

The 2026-10-04 walks start on the way to the start line and have heel stamps and
turns between the legs, so their legs are given as time windows (TEST-058). The
2026-10-02 ones are split at stills of 2 s or more. Results are in docs/testing.md.
"""
import pathlib
import statistics
import subprocess
import sys

RECORDINGS = pathlib.Path(__file__).resolve().parents[2] / "recordings"
# (file stem, replay flags, legs as (start s, end s, counted landings or None)),
# recordings/README.md. None: split at stills, every leg with the count given.
WALKS = [
    ("walk10m-normal1-2026-10-04", ["--still-from", "8"], [(18.5, 29.6, 8), (36.1, 50.8, 8)]),
    ("walk10m-normal2-2026-10-04", ["--still-from", "7"], [(16.1, 27.3, 8), (38.5, 49.3, 8)]),
    ("walk10m-normal3-2026-10-04", ["--still-from", "0"], [(19.3, 29.7, 8), (41.6, 51.3, 8)]),
    ("walk10m-slow1-2026-10-04", ["--still-from", "51.5"], [(15.8, 30.2, 10), (38.2, 50.6, 9)]),
    ("walk10m-slow2-2026-10-04", ["--still-from", "33"], [(14.0, 28.0, 9), (37.0, 50.4, 8)]),
    # Out, turn and back with no stand between: 7 + 7 counted, plus the turn.
    ("walk10m-fast1-2026-10-04", ["--still-from", "37"], [(16.5, 35.9, None)]),
    ("walk10m-normal1-2026-10-02", ["--still-seconds", "5"], 9),
    ("walk10m-normal2-2026-10-02", ["--still-seconds", "5"], 9),
    ("walk10m-normal3-2026-10-02", ["--still-seconds", "5"], 9),
    ("walk10m-slow-2026-10-02", ["--still-seconds", "5"], 11),
    ("walk10m-fast-2026-10-02", ["--still-seconds", "5"], 8),
]


def split_at_stills(events, landings):
    stills, start = [], None
    for t, e in events:
        if e == "zupt_start":
            start = t
        elif e == "zupt_end" and start is not None:
            if t - start >= 2:
                stills.append((start, t))
            start = None
    if start is not None:
        stills.append((start, float("inf")))
    legs = [(stills[i][1], stills[i + 1][0]) for i in range(len(stills) - 1)]
    return [(a, b, landings) for a, b in legs
            if sum(1 for t, e in events if a <= t <= b and e == "toe_off") >= 4]


def main():
    binary, flags = sys.argv[1], sys.argv[2:]
    error = counted = 0
    for name, own_flags, legs in WALKS:
        lines = subprocess.run([binary, str(RECORDINGS / f"{name}.eadlog"), "--trace", *own_flags,
                                *flags], capture_output=True, text=True, check=True).stdout.splitlines()
        events = [(float(l.split()[1]), l.split()[2]) for l in lines if l.startswith("EVENT")]
        cycles = [l.split() for l in lines if l[:6].strip().isdigit()]
        contacts = [t for t, e in events if e == "initial_contact"]
        if not isinstance(legs, list):
            legs = split_at_stills(events, legs)
        row = []
        for a, b, landings in legs:
            found = sum(1 for t in contacts if a <= t <= b)
            # Cycle k opens at contact k; the last contact opens none.
            metres = sum(float(c[4]) for c, t in zip(cycles, contacts)
                         if a <= t <= b and c[-1] == "yes")
            row.append(f"IC {found:2}/{landings if landings else '-'} {metres:5.2f} m")
            if landings:
                error += abs(found - landings)
                counted += landings
        valid = [c for c in cycles if c[-1] == "yes"]
        short = sum(1 for c in valid if float(c[1]) < 0.8)
        median = statistics.median(float(c[4]) for c in valid)
        print(f"{name:27} {' | '.join(row)}  valid {len(valid)}, under 0.8 s {short}, "
              f"median {median:.2f} m")
    print(f"sum |contacts - counted| = {error} over {counted} counted landings")


if __name__ == "__main__":
    main()
