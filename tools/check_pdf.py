#!/usr/bin/env python3
"""Check an exported report.pdf against doc 10 §8.

The report is composed at fixed coordinates with no layout engine, so the thing
that can go wrong is not malformed PDF but content that fell off the page or was
never drawn. `pdftotext` is the independent reader: if a section is not in its
output, it is not on the page.

Run:  python3 tools/check_pdf.py <export directory>
"""
import pathlib
import subprocess
import sys

# Doc 10 §8's list, as it appears on the page.
SECTIONS = [
    "EAD V1 gait session report",
    "Research and engineering report. Not a validated clinical diagnostic report.",
    "Session",
    "Patient",
    "Reference profile",
    "Session measures",
    "Mean speed",
    "Cadence (median)",
    "Stance ratio (median)",
    "Swing ratio (median)",
    "Unilateral cycle symmetry proxy",
    "ZUPT quality (median)",
    "Error score distribution",
    "Error classes",
    "Trends across the session",
    "Event timeline",
    "Segments",
    "Data quality",
    "Haptics: no ERM drivers are fitted",
]

# The seven trend plots doc 10 §8 requires.
TRENDS = [
    "Cadence (steps/min)",
    "Unilateral cycle symmetry proxy",
    "Error score",
    "Stance ratio",
    "Peak dorsiflexion (deg)",
    "Haptic response",
    "ZUPT quality",
]


def main(directory):
    report = pathlib.Path(directory) / "report.pdf"
    failures = []

    def check(condition, message):
        print(f"{'ok  ' if condition else 'FAIL'}  {message}")
        if not condition:
            failures.append(message)

    info = subprocess.run(["pdfinfo", report], capture_output=True, text=True, check=True).stdout
    pages = int(next(l for l in info.splitlines() if l.startswith("Pages:")).split()[1])
    check(pages == 3, f"three pages, got {pages}")
    check("595.28 x 841.89" in info, "A4")
    check("Not a validated clinical" in info, "the subject says what the document is not")

    text = subprocess.run(
        ["pdftotext", "-layout", report, "-"], capture_output=True, text=True, check=True
    ).stdout
    for section in SECTIONS:
        check(section in text, f"section present: {section!r}")

    # The trend page is page 2; every plot must be titled on it.
    page_two = text.split("\f")[1] if "\f" in text else text
    for trend in TRENDS:
        check(trend in page_two, f"trend plot present: {trend!r}")

    # Every page carries the label, so a page printed on its own still says so.
    for index, page in enumerate(text.split("\f")[:3], start=1):
        check(
            "Not a validated clinical diagnostic report." in page,
            f"page {index} carries the research-report label",
        )

    print(f"\n{len(failures)} failures")
    return 1 if failures else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        raise SystemExit(2)
    raise SystemExit(main(sys.argv[1]))
