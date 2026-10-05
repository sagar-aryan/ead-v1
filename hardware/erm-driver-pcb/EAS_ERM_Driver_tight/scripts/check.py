#!/usr/bin/env python3
"""Check the saved PCB against handoff 05 "Required verification" (items 3-15) and print
the connection summary and wire-pad map. Exit code 1 if any check fails.

Usage: python3 scripts/check.py [path/to/EAS_ERM_Driver_tight.kicad_pcb]
Items 1-2 (ERC, DRC incl. schematic parity) are run by scripts/export.sh.
"""
import math
import os
import re
import sys
from collections import defaultdict

import pcbnew

DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
b = pcbnew.LoadBoard(sys.argv[1] if len(sys.argv) > 1 else f"{DIR}/EAS_ERM_Driver_tight.kicad_pcb")
SW, GND, RA, RB = "SWITCHED_SYSTEM+", "GND", "HAPTIC_3V3_A", "HAPTIC_3V3_B"
mm = pcbnew.ToMM
fails = []


def check(ok, msg):
    print(("PASS  " if ok else "FAIL  ") + msg)
    if not ok:
        fails.append(msg)


fps = {f.GetReference(): f for f in b.GetFootprints()}


def net(ref, pad):
    return next(p.GetNetname() for p in fps[ref].Pads() if p.GetNumber() == pad)


def dist(a, b_):
    pa, pb = fps[a].GetPosition(), fps[b_].GetPosition()
    return math.hypot(mm(pa.x - pb.x), mm(pa.y - pb.y))


def rail(n):
    return RA if n <= 3 else RB


# 3 one copper layer / 4 no vias, no holes
tracks = list(b.GetTracks())
vias = [t for t in tracks if t.Type() == pcbnew.PCB_VIA_T]
check(not vias, f"no vias ({len(vias)} found)")
check(all(t.GetLayer() == pcbnew.F_Cu for t in tracks), "every track is on F.Cu")
zones = [z for z in b.Zones() if not z.GetIsRuleArea()]
check(all(z.GetLayerSet().Seq() and all(l == pcbnew.F_Cu for l in z.GetLayerSet().Seq()) for z in zones),
      "every copper zone is on F.Cu")
pads = [p for f in fps.values() for p in f.Pads()]
check(all(p.GetAttribute() == pcbnew.PAD_ATTRIB_SMD for p in pads), "every pad is SMD (no plated/unplated holes)")
check(all(p.GetDrillSize().x == 0 for p in pads), "no pad has a drill")
check(all(not p.GetLayerSet().Contains(pcbnew.B_Cu) for p in pads), "no pad copper on B.Cu")
check(all(not f.IsFlipped() for f in fps.values()), "all footprints on the top (copper) side")
check(not any(d.GetLayer() == pcbnew.B_Cu for d in b.GetDrawings()), "no graphics on B.Cu")

# 5 rails isolated
names = {str(n) for n in b.GetNetsByName().keys() if str(n)}
check(RA in names and RB in names and RA != RB, "HAPTIC_3V3_A and HAPTIC_3V3_B exist as separate nets")
check(net("U1", "3") == RA and net("U2", "3") == RB, "U1 VOUT -> rail A, U2 VOUT -> rail B")
for u in ("U1", "U2"):
    check(net(u, "1") == GND and net(u, "2") == SW, f"{u} pin1=GND pin2(VIN, tab)=SWITCHED_SYSTEM+")

for n in range(1, 7):
    r, m, g, pwm = rail(n), f"M{n}-", f"GATE{n}", f"PWM{n}"
    check(net(f"Q{n}", "2") == GND, f"Q{n} source -> GND")                                    # 6
    check(net(f"Q{n}", "3") == m, f"Q{n} drain -> {m}")
    check(net(f"D{n}", "1") == r and net(f"D{n}", "2") == m, f"D{n} cathode -> {r}, anode -> {m}")  # 7
    check({net(f"C{n + 8}", "1"), net(f"C{n + 8}", "2")} == {r, m}, f"C{n + 8} (10 nF) across {r} / {m}")  # 8
    check({net(f"C{n + 14}", "1"), net(f"C{n + 14}", "2")} == {r, GND}, f"C{n + 14} (100 nF) {r} to GND")
    rg, rpd = fps[f"R{n}"], fps[f"R{n + 6}"]
    check(rg.GetValue() == "100" and {net(f"R{n}", "1"), net(f"R{n}", "2")} == {pwm, g}
          and net(f"Q{n}", "1") == g, f"{pwm} -> R{n} 100R -> Q{n} gate")                   # 9
    check(rpd.GetValue() == "100k" and {net(f"R{n + 6}", "1"), net(f"R{n + 6}", "2")} == {g, GND},
          f"Q{n} gate -> R{n + 6} 100k -> GND")                                              # 10
    for part, lim in ((f"D{n}", 6.0), (f"C{n + 8}", 6.0), (f"C{n + 14}", 6.0), (f"R{n}", 6.0), (f"R{n + 6}", 6.0)):
        check(dist(part, f"Q{n}") <= lim, f"{part} within {lim} mm of Q{n} ({dist(part, f'Q{n}'):.1f} mm)")

# 11 / 12 supply nets
esp = {f.GetValue(): f for r, f in fps.items() if r.startswith("J")}
check(net(esp["ESP_SW_BAT+"].GetReference(), "1") == SW, "ESP_SW_BAT+ pad is on SWITCHED_SYSTEM+ (not a 3.3 V net)")
check(not any(re.search(r"BAT", n) for n in names), "no BAT-named net enters the PCB")

# 14 bulk capacitor polarity (pad 1 = +)
check(net("C7", "1") == RA and net("C7", "2") == GND, "C7 220 uF: + on rail A, - on GND")
check(net("C8", "1") == RB and net("C8", "2") == GND, "C8 220 uF: + on rail B, - on GND")
# 220 uF bulk caps only carry the low-frequency motor current (HF: C5/C6 and C15-C20), so 10 mm.
for u, caps in (("U1", {"C1": 6, "C3": 9, "C5": 6, "C7": 10}), ("U2", {"C2": 9, "C4": 9, "C6": 6, "C8": 10})):
    for c, lim in caps.items():
        check(dist(c, u) <= lim, f"{c} within {lim} mm of {u} ({dist(c, u):.1f} mm)")

# 13 copper widths
w = defaultdict(list)
for t in tracks:
    w[t.GetNetname()].append(round(mm(t.GetWidth()), 3))
for n_, ws in sorted(w.items()):
    if n_ in (SW, GND):
        ok = min(ws) >= 0.8 and max(ws) >= 1.0  # mini variant: 1.0 mm trunks (DEC-017)
        rule = "branches >= 0.8, trunk >= 1.0"
    elif n_ in (RA, RB):
        ok, rule = min(ws) >= 1.0, ">= 1.0"
    elif n_.startswith("M"):
        ok, rule = min(ws) >= 0.8, ">= 0.8"
    else:
        ok, rule = 0.4 <= min(ws) and max(ws) <= 0.5, "0.4-0.5"
    check(ok, f"track width {n_}: {min(ws)}-{max(ws)} mm ({rule})")

# wire pads
jp = sorted((f for r, f in fps.items() if r.startswith("J")), key=lambda f: int(f.GetReference()[1:]))
check(len(jp) == 18, f"18 wire pads ({len(jp)} found)")
for f in jp:
    p = f.Pads()[0]
    sx, sy = sorted((mm(p.GetSize(pcbnew.F_Cu).x), mm(p.GetSize(pcbnew.F_Cu).y)))
    need = (2.0, 2.0) if f.GetValue().startswith(("ESP", "PWM")) else (2.5, 2.5) if f.GetValue()[0] in "M" else (2.5, 3.0)
    check(sx >= need[0] and sy >= need[1], f"{f.GetReference()} {f.GetValue()} pad {sx}x{sy} mm >= {need[0]}x{need[1]}")

edge = next(d for d in b.GetDrawings() if d.GetLayer() == pcbnew.Edge_Cuts)
bb = pcbnew.BOX2I(edge.GetStart(), edge.GetEnd() - edge.GetStart())
print(f"\nBoard outline (cut line): {mm(bb.GetWidth()):.2f} x {mm(bb.GetHeight()):.2f} mm")

print("\nWire pad map (board-local mm, origin = top-left of outline):")
ox, oy = bb.GetLeft(), bb.GetTop()
for f in jp:
    p = f.Pads()[0]
    s = p.GetSize(pcbnew.F_Cu)
    rot = int(round(f.GetOrientationDegrees())) % 180
    sx, sy = (mm(s.y), mm(s.x)) if rot == 90 else (mm(s.x), mm(s.y))
    print(f"  {f.GetReference():4} {f.GetValue():17} net {p.GetNetname():17} at ({mm(p.GetPosition().x - ox):5.1f},"
          f" {mm(p.GetPosition().y - oy):4.1f})  {sx:.1f} x {sy:.1f} mm")

print("\nConnection summary:")
members = defaultdict(list)
for f in sorted(fps.values(), key=lambda f: (re.sub(r"\d", "", f.GetReference()), int(re.sub(r"\D", "", f.GetReference())))):
    for p in f.Pads():
        members[p.GetNetname()].append(f"{f.GetReference()}.{p.GetNumber()}")
for n_ in sorted(members):
    print(f"  {n_:17} {' '.join(members[n_])}")

print(f"\n{'ALL CHECKS PASSED' if not fails else f'{len(fails)} CHECK(S) FAILED'}")
sys.exit(1 if fails else 0)
