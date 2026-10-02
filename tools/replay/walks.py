#!/usr/bin/env python3
"""Replays the 10 m walk fixtures and compares each leg with its ground truth.

  walks.py /path/to/eadreplay [replay flags...]

Per leg (legs are split at stills of 2 s or more): toe-offs, contacts against the
right-foot landings the walker counted, and the summed distance of its valid
cycles. A leg's cycles start at its first detected contact, so they cover 10 m
less the first step from standing. Results are recorded in docs/testing.md.
"""
import pathlib
import statistics
import subprocess
import sys

RECORDINGS = pathlib.Path(__file__).resolve().parents[2] / "recordings"
# (file stem, right-foot landings counted per leg), recordings/README.md
WALKS = [("normal1", 9), ("normal2", 9), ("normal3", 9), ("slow", 11), ("fast", 8)]


def main():
    binary, flags = sys.argv[1], sys.argv[2:]
    contacts = counted = 0
    for name, landings in WALKS:
        path = RECORDINGS / f"walk10m-{name}-2026-10-02.eadlog"
        lines = subprocess.run([binary, str(path), "--still-seconds", "5", "--trace", *flags],
                               capture_output=True, text=True, check=True).stdout.splitlines()
        events = [(float(l.split()[1]), l.split()[2]) for l in lines if l.startswith("EVENT")]
        cycles = [l.split() for l in lines if l[:6].strip().isdigit()]
        ics = [t for t, e in events if e == "initial_contact"]
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
        legs = [(a, b) for a, b in legs if sum(1 for t, e in events if a <= t <= b and e == "toe_off") >= 4]
        row = []
        for a, b in legs:
            toe_offs = sum(1 for t, e in events if a <= t <= b and e == "toe_off")
            found = sum(1 for t in ics if a <= t <= b)
            # Cycle k opens at contact k.
            metres = sum(float(c[4]) for c, t in zip(cycles, ics) if a <= t <= b and c[-1] == "yes")
            row.append(f"TO {toe_offs:2} IC {found:2}/{landings} {metres:5.2f} m")
            contacts += found
            counted += landings
        valid = [c for c in cycles if c[-1] == "yes"]
        short = sum(1 for c in valid if float(c[1]) < 0.9)
        uncorrected = sum(1 for c in valid if float(c[6]) < 0.03)
        median = statistics.median(float(c[4]) for c in valid)
        print(f"{name:8} {' | '.join(row)}  valid {len(valid)}, under 0.9 s {short}, "
              f"no ZUPT {uncorrected}, median {median:.2f} m")
    print(f"contacts {contacts} of {counted} counted landings")


if __name__ == "__main__":
    main()
